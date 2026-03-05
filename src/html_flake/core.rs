// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use crate::{
    compiler::section::HTMLContent,
    entry::{EntryMetaData, MetaData},
    environment,
    html_macro::html,
    slug::Slug,
};

pub fn html_article_inner(
    metadata: &EntryMetaData,
    contents: &String,
    hide_metadata: bool,
    open: bool,
    adhoc_title: Option<&str>,
    adhoc_taxon: Option<&str>,
) -> eyre::Result<String> {
    let summary = metadata.to_header(adhoc_title, adhoc_taxon)?;

    let article_id = metadata.id()?;
    Ok(html_section(
        &summary,
        contents,
        hide_metadata,
        open,
        article_id,
        metadata.data_taxon(),
    ))
}

pub fn html_footer_section(id: &str, summary: &str, content: &String) -> String {
    let summary = format!("<header><h1>{}</h1></header>", summary);
    let inner_html = format!("{}{}", (html!(summary { (summary) })), content);
    let html_details = format!("<details open>{}</details>", inner_html);
    html!(section class="block link-list" id={id} { (html_details) })
}

pub fn html_section(
    summary: &String,
    content: &String,
    hide_metadata: bool,
    open: bool,
    id: String,
    data_taxon: Option<&String>,
) -> String {
    let mut class_name: Vec<&str> = vec!["block"];
    if hide_metadata {
        class_name.push("hide-metadata");
    }
    let data_taxon = data_taxon.map_or("", |s| s);
    let open = if open { "open" } else { "" };
    let inner_html = format!("{}{}", (html!(summary id={id} { (summary) })), content);
    let html_details = format!("<details {}>{}</details>", open, inner_html);
    html!(section class={class_name.join(" ")} data_taxon={data_taxon} { (html_details) })
}

pub fn catalog_item(
    slug: Slug,
    title: &str,
    page_title: &str,
    details_open: bool,
    taxon: &str,
    child_html: &str,
) -> String {
    let slug_url = environment::full_html_url(slug);
    let title_text = format!("{} [{}]", page_title, slug);
    let onclick = format!(
        "window.location.href='#{}'",
        crate::slug::to_hash_id(slug.as_str())
    );

    let mut class_name: Vec<String> = vec![];
    if !details_open {
        class_name.push("item-summary".to_string());
    }

    html!(li class={class_name.join(" ")} {
        a class="bullet" href={slug_url} title={title_text} { "■" }
        span class="link local" onclick={onclick} {
            span class="taxon" { (taxon) }
            (title)
        }
        (child_html)
    })
}

pub fn html_catalog_block(items: &str) -> String {
    let toc_text = environment::get_toc_text();
    html!(div class="block" {
        details open="" { summary { h1 { (toc_text) } } (items) }
    })
}

pub fn html_inline_typst_span(svg: &str) -> String {
    html!(span class="inline-typst" { (svg) })
}

pub fn html_footer(references_html: &str, backlinks_html: &str) -> String {
    html!(footer { (references_html) (backlinks_html) })
}

pub fn footnote_reference(s: &str, back_id: &str, number: usize) -> String {
    html!(sup class="footnote-reference" id={back_id} {
      a href={format!("#{}", s)} { (number) }
    })
}

pub fn html_image(image_src: &str, class_name: &str) -> String {
    format!(r#"<img src="{image_src}" class="{class_name}"/>"#)
}

pub fn html_image_color_invert(image_src: &str) -> String {
    html_image(image_src, "color-invert")
}

pub fn html_figure(image_src: &str, is_block: bool, caption: String) -> String {
    if !is_block {
        return html!(span class="inline-typst" { (html_image_color_invert(image_src)) });
    }
    let mut caption = caption;
    if !caption.is_empty() {
        caption = html!(figcaption { (caption) })
    }
    html!(figure { (html_image_color_invert(image_src)) (caption) })
}

pub fn html_figure_code(image_src: &str, caption: String, code: String) -> String {
    let mut caption = caption;
    if !caption.is_empty() {
        caption = html!(figcaption { (caption) })
    }
    let figure = html!(figure { (html_image_color_invert(image_src)) (caption) });
    let pre = html!(pre { (code) });
    html!(details { summary { (figure) } (pre) })
}

pub fn html_link(href: &str, title: &str, text: &str, class_name: &str) -> String {
    let plain_title = HTMLContent::Plain(title.to_string()).remove_all_tags();
    let escaped_href = htmlize::escape_attribute(href);
    let escaped_title = htmlize::escape_attribute(&plain_title);
    let escaped_class = htmlize::escape_attribute(class_name);
    format!(
        r#"<span class="link {}"><a href="{}" title="{}">{}</a></span>"#,
        escaped_class, escaped_href, escaped_title, text
    )
}

/// Also see [`crate::compiler::parser::tests::test_code_block`]
pub fn html_code_block(code: &str, language: &str) -> String {
    html!(pre { code class={format!("language-{}", language)} { (code) } })
}

pub fn html_header_nav(title: &str, page_title: &str, href: &str) -> String {
    let onclick = format!("window.location.href='{}'", href);
    html!(header class="header" {
        nav class="nav" {
            div class="logo" {
                span class="cursor-pointer" onclick={onclick} title={page_title} {
                    "« " (title)
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::html_link;

    #[test]
    fn test_html_link_escapes_title_attribute() {
        let html = html_link(
            "/AC2C",
            r#"<span lang="zh">abc</span> [AC2C]""#,
            r#"<span lang="zh">abc</span>"#,
            "local",
        );
        assert!(html.contains(r#"href="/AC2C""#));
        assert!(html.contains(r#"title="abc [AC2C]&quot;""#));
        assert!(!html.contains("&lt;span"));
        assert!(!html.contains("&lt;/span&gt;"));
        assert!(!html.contains(r#"title="<span lang="zh">"#));
        assert!(html.contains(r#"><span lang="zh">abc</span></a>"#));
    }
}
