// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use eyre::eyre;
use url::Url;

use crate::{entry::MetaData, environment, slug::Slug};

use super::{state::CompileState, writer::Writer};

#[derive(Debug, Clone)]
struct FeedItem {
    slug: Slug,
    title: String,
    link: String,
    date: String,
    summary_text: String,
    content_html: String,
}

const DESCRIPTION_MAX_CHARS: usize = 280;

pub(super) fn ensure_publish_rss_base_url_is_absolute() -> eyre::Result<()> {
    validate_publish_rss_base_url(&environment::base_url_raw())
}

fn validate_publish_rss_base_url(base_url: &str) -> eyre::Result<()> {
    let base_url = base_url.trim();
    let Ok(url) = Url::parse(base_url) else {
        return Err(eyre!(
            "invalid `[kodama].base-url` for RSS publish: expected absolute `http://` or `https://` URL, got `{base_url}`. \
set `[kodama].base-url = \"https://example.com/\"`."
        ));
    };

    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(eyre!(
            "invalid `[kodama].base-url` for RSS publish: expected absolute `http://` or `https://` URL, got `{base_url}`. \
set `[kodama].base-url = \"https://example.com/\"`."
        ));
    }

    Ok(())
}

pub(super) fn feed_xml(state: &CompileState) -> eyre::Result<String> {
    let channel_title = channel_title(state);
    let channel_link = environment::full_url("/");
    let channel_self_link = environment::full_url("/feed.xml");

    let mut items = collect_items(state)?;
    sort_feed_items(&mut items);

    let mut output = String::new();
    output.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    output.push('\n');
    output.push_str(
        r#"<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom" xmlns:content="http://purl.org/rss/1.0/modules/content/">"#,
    );
    output.push('\n');
    output.push_str("  <channel>\n");
    push_tag(&mut output, 4, "title", &channel_title);
    push_tag(&mut output, 4, "link", &channel_link);
    push_atom_self_link(&mut output, &channel_self_link);
    push_tag(
        &mut output,
        4,
        "description",
        &format!("RSS feed for {}", channel_title),
    );
    if let Some(last_build_date) = last_build_date(&items) {
        push_tag(&mut output, 4, "lastBuildDate", &last_build_date);
    }

    for item in items {
        output.push_str("    <item>\n");
        push_tag(&mut output, 6, "title", &item.title);
        push_tag(&mut output, 6, "link", &item.link);
        push_guid_tag(&mut output, 6, item.slug.as_str());
        if let Some(pub_date) = normalize_pub_date(&item.date) {
            push_tag(&mut output, 6, "pubDate", &pub_date);
        }
        if !item.summary_text.is_empty() {
            push_tag(&mut output, 6, "description", &item.summary_text);
        }
        if !item.content_html.trim().is_empty() {
            push_cdata_tag(&mut output, 6, "content:encoded", &item.content_html);
        }
        output.push_str("    </item>\n");
    }

    output.push_str("  </channel>\n");
    output.push_str("</rss>\n");
    Ok(output)
}

fn collect_items(state: &CompileState) -> eyre::Result<Vec<FeedItem>> {
    let index_slug = Slug::new("index");
    let mut items = Vec::new();

    for (&slug, section) in state.compiled() {
        if slug == index_slug || section.metadata.is_collect()? {
            continue;
        }

        let title = section
            .metadata
            .page_title()
            .or_else(|| section.metadata.title())
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .unwrap_or(slug.as_str())
            .to_string();
        let date = section
            .metadata
            .get_str("date")
            .cloned()
            .unwrap_or_default();
        let link = environment::full_html_url(slug);
        let content_html = Writer::rss_content_html(section, state)?;
        let summary_text = summary_text_from_html(&content_html);

        items.push(FeedItem {
            slug,
            title,
            link,
            date,
            summary_text,
            content_html,
        });
    }

    Ok(items)
}

fn summary_text_from_html(content_html: &str) -> String {
    let text = strip_html_tags(content_html);
    let collapsed = collapse_whitespace(&text);
    truncate_to_max_chars(collapsed.trim(), DESCRIPTION_MAX_CHARS)
}

fn strip_html_tags(value: &str) -> String {
    crate::compiler::section::HTMLContent::Plain(value.to_string()).remove_all_tags()
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_to_max_chars(value: &str, max_chars: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_chars {
        return value.to_string();
    }

    if max_chars == 0 {
        return String::new();
    }

    if max_chars <= 3 {
        return value.chars().take(max_chars).collect();
    }

    let mut out = String::new();
    for ch in value.chars().take(max_chars - 3) {
        out.push(ch);
    }
    out.push_str("...");
    out
}

fn sort_feed_items(items: &mut [FeedItem]) {
    items.sort_by(|left, right| {
        crate::footer_sort::compare_values("date", left.date.as_str(), right.date.as_str())
            .reverse()
            .then_with(|| left.slug.cmp(&right.slug))
    });
}

fn channel_title(state: &CompileState) -> String {
    state
        .compiled()
        .get(&Slug::new("index"))
        .and_then(|section| {
            section
                .metadata
                .page_title()
                .or_else(|| section.metadata.title())
        })
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("Kodama Feed")
        .to_string()
}

fn last_build_date(items: &[FeedItem]) -> Option<String> {
    items
        .iter()
        .filter_map(|item| {
            let date = crate::footer_sort::parse_date(item.date.trim())?;
            let formatted = format_rfc822_date(date)?;
            Some((date, formatted))
        })
        .max_by_key(|(date, _)| *date)
        .map(|(_, formatted)| formatted)
}

fn normalize_pub_date(date: &str) -> Option<String> {
    let date = date.trim();
    if date.is_empty() {
        return None;
    }

    Some(
        crate::footer_sort::parse_date(date)
            .and_then(format_rfc822_date)
            .unwrap_or_else(|| date.to_string()),
    )
}

fn format_rfc822_date((year, month, day): (u32, u8, u8)) -> Option<String> {
    if !is_valid_calendar_date(year, month, day) {
        return None;
    }

    let weekday = weekday_name(day_of_week(year, month, day));
    let month = month_abbr(month);
    Some(format!(
        "{weekday}, {day:02} {month} {year:04} 00:00:00 GMT"
    ))
}

fn is_valid_calendar_date(year: u32, month: u8, day: u8) -> bool {
    let max_day = days_in_month(year, month);
    max_day != 0 && day != 0 && day <= max_day
}

fn days_in_month(year: u32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn is_leap_year(year: u32) -> bool {
    year.is_multiple_of(400) || (year.is_multiple_of(4) && !year.is_multiple_of(100))
}

fn day_of_week(year: u32, month: u8, day: u8) -> usize {
    let mut y = year as i32;
    let mut m = month as i32;
    if m < 3 {
        y -= 1;
        m += 12;
    }

    let q = day as i32;
    let k = y % 100;
    let j = y / 100;
    let h = (q + ((13 * (m + 1)) / 5) + k + (k / 4) + (j / 4) + (5 * j)) % 7;
    ((h + 6) % 7) as usize
}

fn weekday_name(index: usize) -> &'static str {
    match index {
        0 => "Sun",
        1 => "Mon",
        2 => "Tue",
        3 => "Wed",
        4 => "Thu",
        5 => "Fri",
        6 => "Sat",
        _ => unreachable!(),
    }
}

fn month_abbr(month: u8) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => unreachable!(),
    }
}

fn push_atom_self_link(output: &mut String, href: &str) {
    output.push_str("    <atom:link href=\"");
    output.push_str(&xml_escape(href));
    output.push_str("\" rel=\"self\" type=\"application/rss+xml\" />\n");
}

fn push_guid_tag(output: &mut String, indent: usize, guid: &str) {
    output.push_str(&" ".repeat(indent));
    output.push_str(r#"<guid isPermaLink="false">"#);
    output.push_str(&xml_escape(guid));
    output.push_str("</guid>\n");
}

fn push_cdata_tag(output: &mut String, indent: usize, tag: &str, value: &str) {
    output.push_str(&" ".repeat(indent));
    output.push('<');
    output.push_str(tag);
    output.push_str("><![CDATA[");
    output.push_str(&escape_cdata(value));
    output.push_str("]]></");
    output.push_str(tag);
    output.push_str(">\n");
}

fn escape_cdata(value: &str) -> String {
    value.replace("]]>", "]]]]><![CDATA[>")
}

fn push_tag(output: &mut String, indent: usize, tag: &str, value: &str) {
    output.push_str(&" ".repeat(indent));
    output.push('<');
    output.push_str(tag);
    output.push('>');
    output.push_str(&xml_escape(value));
    output.push_str("</");
    output.push_str(tag);
    output.push_str(">\n");
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        compiler::{
            section::{EmbedContent, HTMLContent, LazyContent, SectionOption, UnresolvedSection},
            state::compile_all_without_missing_index_warning,
        },
        entry::{HTMLMetaData, KEY_COLLECT, KEY_EXT, KEY_PAGE_TITLE, KEY_SLUG},
        ordered_map::OrderedMap,
    };

    use super::*;

    fn shallow(
        slug: &str,
        page_title: &str,
        date: Option<&str>,
        content_html: &str,
    ) -> UnresolvedSection {
        let mut metadata = OrderedMap::new();
        metadata.insert(KEY_SLUG.to_string(), HTMLContent::Plain(slug.to_string()));
        metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain("md".to_string()));
        metadata.insert(
            KEY_PAGE_TITLE.to_string(),
            HTMLContent::Plain(page_title.to_string()),
        );
        if let Some(date) = date {
            metadata.insert("date".to_string(), HTMLContent::Plain(date.to_string()));
        }

        UnresolvedSection {
            metadata: HTMLMetaData(metadata),
            content: HTMLContent::Plain(content_html.to_string()),
        }
    }

    fn compile_state_for_feed(item_date: &str, item_content: &str) -> CompileState {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("index"),
            shallow("index", "Site", Some("2020-01-01"), "<p>index page</p>"),
        );
        shallows.insert(
            Slug::new("post"),
            shallow("post", "Post", Some(item_date), item_content),
        );
        compile_all_without_missing_index_warning(&shallows).unwrap()
    }

    #[test]
    fn test_sort_feed_items_uses_parsed_date_desc() {
        let mut items = vec![
            FeedItem {
                slug: Slug::new("old"),
                title: "Old".to_string(),
                link: "https://example.com/old".to_string(),
                date: "January 2, 2020".to_string(),
                summary_text: String::new(),
                content_html: String::new(),
            },
            FeedItem {
                slug: Slug::new("mid"),
                title: "Mid".to_string(),
                link: "https://example.com/mid".to_string(),
                date: "2021-01-01".to_string(),
                summary_text: String::new(),
                content_html: String::new(),
            },
            FeedItem {
                slug: Slug::new("new"),
                title: "New".to_string(),
                link: "https://example.com/new".to_string(),
                date: "August 15, 2021".to_string(),
                summary_text: String::new(),
                content_html: String::new(),
            },
        ];

        sort_feed_items(&mut items);

        assert_eq!(items[0].slug, Slug::new("new"));
        assert_eq!(items[1].slug, Slug::new("mid"));
        assert_eq!(items[2].slug, Slug::new("old"));
    }

    #[test]
    fn test_normalize_pub_date_prefers_rfc822_when_parseable() {
        let formatted = normalize_pub_date("2021-08-15");
        assert_eq!(formatted, Some("Sun, 15 Aug 2021 00:00:00 GMT".to_string()));
    }

    #[test]
    fn test_normalize_pub_date_keeps_raw_when_unparseable() {
        let formatted = normalize_pub_date("not-a-date");
        assert_eq!(formatted, Some("not-a-date".to_string()));
    }

    #[test]
    fn test_feed_xml_includes_atom_self_link_and_last_build_date() {
        let state = compile_state_for_feed("2021-08-15", "<p>Hello <strong>world</strong></p>");
        let xml = feed_xml(&state).unwrap();
        let self_link = environment::full_url("/feed.xml");

        assert!(xml.contains(
            r#"<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom" xmlns:content="http://purl.org/rss/1.0/modules/content/">"#
        ));
        assert!(xml.contains(&format!(
            r#"<atom:link href="{}" rel="self" type="application/rss+xml" />"#,
            xml_escape(&self_link),
        )));
        assert!(xml.contains("<lastBuildDate>Sun, 15 Aug 2021 00:00:00 GMT</lastBuildDate>"));
    }

    #[test]
    fn test_feed_xml_keeps_raw_pub_date_when_unparseable() {
        let state = compile_state_for_feed("not-a-date", "<p>Hello</p>");
        let xml = feed_xml(&state).unwrap();
        assert!(xml.contains("<pubDate>not-a-date</pubDate>"));
        assert!(!xml.contains("<lastBuildDate>"));
    }

    #[test]
    fn test_feed_xml_uses_slug_guid_with_non_permalink_flag() {
        let state = compile_state_for_feed("2021-08-15", "<p>Hello</p>");
        let xml = feed_xml(&state).unwrap();
        let post_link = environment::full_html_url(Slug::new("post"));
        assert!(xml.contains(r#"<guid isPermaLink="false">post</guid>"#));
        assert!(xml.contains(&format!("<link>{post_link}</link>")));
    }

    #[test]
    fn test_feed_xml_contains_description_and_content_encoded() {
        let state = compile_state_for_feed("2021-08-15", "<p>Hello <strong>world</strong></p>");
        let xml = feed_xml(&state).unwrap();
        let items = collect_items(&state).unwrap();
        let item = items
            .iter()
            .find(|item| item.slug == Slug::new("post"))
            .expect("post item exists");

        assert!(!item.summary_text.is_empty());
        assert!(item.content_html.contains("<strong>world</strong>"));
        assert!(xml.contains(&format!(
            "<description>{}</description>",
            xml_escape(&item.summary_text)
        )));
        assert!(xml.contains(&format!(
            "<content:encoded><![CDATA[{}]]></content:encoded>",
            escape_cdata(&item.content_html)
        )));
    }

    #[test]
    fn test_feed_xml_excludes_index_from_items() {
        let state = compile_state_for_feed("2021-08-15", "<p>Hello</p>");
        let xml = feed_xml(&state).unwrap();
        assert!(!xml.contains(r#"<guid isPermaLink="false">index</guid>"#));
        assert!(xml.contains(r#"<guid isPermaLink="false">post</guid>"#));
    }

    #[test]
    fn test_content_encoded_splits_cdata_terminator() {
        let state = compile_state_for_feed("2021-08-15", "<p>a ]]> b</p>");
        let xml = feed_xml(&state).unwrap();
        let items = collect_items(&state).unwrap();
        let item = items
            .iter()
            .find(|item| item.slug == Slug::new("post"))
            .expect("post item exists");
        let escaped = escape_cdata(&item.content_html);

        assert!(escaped.contains("]]]]><![CDATA[>"));
        assert!(xml.contains(&format!(
            "<content:encoded><![CDATA[{escaped}]]></content:encoded>"
        )));
    }

    #[test]
    fn test_feed_xml_handles_embedded_sections_without_panicking() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("index"),
            shallow("index", "Site", Some("2020-01-01"), "<p>index page</p>"),
        );
        shallows.insert(
            Slug::new("post"),
            shallow("post", "Post", Some("2021-08-15"), ""),
        );
        shallows.insert(
            Slug::new("child"),
            shallow("child", "Child", Some("2021-08-14"), "<p>child body</p>"),
        );
        if let Some(post) = shallows.get_mut(&Slug::new("post")) {
            post.content = HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                url: "/child".to_string(),
                title: None,
                option: SectionOption::default(),
            })]);
        }

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();
        let xml = feed_xml(&state).unwrap();
        assert!(xml.contains(r#"<guid isPermaLink="false">post</guid>"#));
        assert!(xml.contains("<content:encoded><![CDATA["));
        assert!(xml.contains("child body"));
    }

    #[test]
    fn test_feed_xml_excludes_collect_sections() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("index"),
            shallow("index", "Site", Some("2020-01-01"), "<p>index page</p>"),
        );
        shallows.insert(
            Slug::new("post"),
            shallow("post", "Post", Some("2021-08-15"), "<p>post</p>"),
        );
        let mut collect = shallow("catalog", "Catalog", Some("2021-08-16"), "<p>catalog</p>");
        collect.metadata.0.insert(
            KEY_COLLECT.to_string(),
            HTMLContent::Plain("true".to_string()),
        );
        shallows.insert(Slug::new("catalog"), collect);

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();
        let xml = feed_xml(&state).unwrap();
        assert!(xml.contains(r#"<guid isPermaLink="false">post</guid>"#));
        assert!(!xml.contains(r#"<guid isPermaLink="false">catalog</guid>"#));
    }

    #[test]
    fn test_summary_text_is_collapsed_and_truncated() {
        let long = format!("<p>{}</p>", "x ".repeat(400));
        let summary = summary_text_from_html(&long);
        assert!(summary.chars().count() <= DESCRIPTION_MAX_CHARS);
        assert!(summary.ends_with("..."));
    }

    #[test]
    fn test_xml_escape_escapes_special_chars() {
        assert_eq!(
            xml_escape(r#"Tom & Jerry <"quote"> 'single'"#),
            "Tom &amp; Jerry &lt;&quot;quote&quot;&gt; &apos;single&apos;"
        );
    }

    #[test]
    fn test_validate_publish_rss_base_url_accepts_absolute_http_url() {
        assert!(validate_publish_rss_base_url("http://example.com/").is_ok());
        assert!(validate_publish_rss_base_url("https://example.com/blog/").is_ok());
    }

    #[test]
    fn test_validate_publish_rss_base_url_rejects_relative_or_non_http_url() {
        assert!(validate_publish_rss_base_url("/").is_err());
        assert!(validate_publish_rss_base_url("./notes").is_err());
        assert!(validate_publish_rss_base_url("example.com").is_err());
        assert!(validate_publish_rss_base_url("ftp://example.com/").is_err());
    }
}
