// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use crate::{entry::MetaData, environment, slug::Slug};

use super::state::CompileState;

#[derive(Debug, Clone)]
struct FeedItem {
    slug: Slug,
    title: String,
    link: String,
    date: String,
}

pub(super) fn feed_xml(state: &CompileState) -> String {
    let channel_title = channel_title(state);
    let channel_link = environment::full_url("/");

    let mut items = collect_items(state);
    sort_feed_items(&mut items);

    let mut output = String::new();
    output.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    output.push('\n');
    output.push_str(r#"<rss version="2.0">"#);
    output.push('\n');
    output.push_str("  <channel>\n");
    push_tag(&mut output, 4, "title", &channel_title);
    push_tag(&mut output, 4, "link", &channel_link);
    push_tag(
        &mut output,
        4,
        "description",
        &format!("RSS feed for {}", channel_title),
    );

    for item in items {
        output.push_str("    <item>\n");
        push_tag(&mut output, 6, "title", &item.title);
        push_tag(&mut output, 6, "link", &item.link);
        push_tag(&mut output, 6, "guid", &item.link);
        if !item.date.trim().is_empty() {
            push_tag(&mut output, 6, "pubDate", &item.date);
        }
        output.push_str("    </item>\n");
    }

    output.push_str("  </channel>\n");
    output.push_str("</rss>\n");
    output
}

fn collect_items(state: &CompileState) -> Vec<FeedItem> {
    state
        .compiled()
        .iter()
        .map(|(&slug, section)| {
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
            FeedItem {
                slug,
                title,
                link,
                date,
            }
        })
        .collect()
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
    use super::*;

    #[test]
    fn test_sort_feed_items_uses_parsed_date_desc() {
        let mut items = vec![
            FeedItem {
                slug: Slug::new("old"),
                title: "Old".to_string(),
                link: "https://example.com/old".to_string(),
                date: "January 2, 2020".to_string(),
            },
            FeedItem {
                slug: Slug::new("mid"),
                title: "Mid".to_string(),
                link: "https://example.com/mid".to_string(),
                date: "2021-01-01".to_string(),
            },
            FeedItem {
                slug: Slug::new("new"),
                title: "New".to_string(),
                link: "https://example.com/new".to_string(),
                date: "August 15, 2021".to_string(),
            },
        ];

        sort_feed_items(&mut items);

        assert_eq!(items[0].slug, Slug::new("new"));
        assert_eq!(items[1].slug, Slug::new("mid"));
        assert_eq!(items[2].slug, Slug::new("old"));
    }

    #[test]
    fn test_xml_escape_escapes_special_chars() {
        assert_eq!(
            xml_escape(r#"Tom & Jerry <"quote"> 'single'"#),
            "Tom &amp; Jerry &lt;&quot;quote&quot;&gt; &apos;single&apos;"
        );
    }
}
