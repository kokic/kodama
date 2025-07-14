// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::ops::Not;

use crate::{
    config::{self, input_path},
    entry::{EntryMetaData, MetaData},
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
) -> String {
    let summary = metadata.to_header(adhoc_title, adhoc_taxon);

    let article_id = metadata.id();
    crate::html_flake::html_section(
        &summary,
        contents,
        hide_metadata,
        open,
        article_id,
        metadata.data_taxon(),
    )
}

pub fn html_footer_section(summary: &str, content: &String) -> String {
    let summary = format!("<header><h1>{}</h1></header>", summary);
    let inner_html = format!("{}{}", (html!(summary { (summary) })), content);
    let html_details = format!("<details open>{}</details>", inner_html);
    html!(section class="block" { (html_details) })
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
    let open = open.then(|| "open").unwrap_or("");
    let inner_html = format!("{}{}", (html!(summary id={id} { (summary) })), content);
    let html_details = format!("<details {}>{}</details>", open, inner_html);
    html!(section class={class_name.join(" ")} data_taxon={data_taxon} { (html_details) })
}

pub fn html_header_metadata(mut etc: Vec<String>) -> String {
    let mut meta_items: Vec<String> = vec![];
    meta_items.append(&mut etc);
    let items = meta_items
        .iter()
        .map(|item| html!(li class="meta-item" { (item) }))
        .reduce(|s: String, t: String| s + t.as_str())
        .unwrap_or(String::new());

    html!(div class="metadata" { ul { (items) } })
}

pub fn html_header(
    title: &str,
    taxon: &str,
    slug: &Slug,
    span_class: String,
    etc: Vec<String>,
) -> String {
    let slug_text = EntryMetaData::to_slug_text(slug.as_str());
    let slug_url = config::full_html_url(*slug);

    let editor_url = match config::editor_url() {
        Some(prefix) if config::is_serve() => {
            // Bug: The suffix of slug is not necessarily `.md`,
            // perhaps we need to add an `ext` field for [`Slug`].
            let source_path = input_path(format!("{}.md", slug.as_str()))
                .canonicalize()
                .unwrap();
            let source_url = url::Url::from_file_path(source_path).unwrap();
            let base = url::Url::parse(&prefix).unwrap();
            let editor_url = base.join(source_url.path()).unwrap();
            html!(a class="slug" href={editor_url.to_string()} { "[edit]" })
        }
        _ => String::default(),
    };

    html!(header {
        h1 {
            span class={span_class} { (taxon) }
            (title) " "
            a class="slug" href={slug_url} { "["(slug_text)"]" }
            (editor_url)
        }
        (html_header_metadata(etc))
    })
}

pub fn catalog_item(
    slug: Slug,
    title: &str,
    page_title: &str,
    details_open: bool,
    taxon: &str,
    child_html: &str,
) -> String {
    let slug_url = config::full_html_url(slug);
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
    html!(div class="block" { h1 { "Table of Contents" } (items) })
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

pub fn html_figure(image_src: &str, center: bool, caption: String) -> String {
    if !center {
        return html_image_color_invert(image_src);
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
    html!(span class={format!("link {}", class_name)} {
        a href={href} title={title} { (text) }
    })
}

pub fn html_header_nav(title: &str, page_title: &str, href: &str) -> String {
    let onclick = format!("window.location.href='{}'", href);
    html!(header class="header" {
        nav class="nav" {
            div class="logo" {
                span onclick={onclick} title={page_title} {
                    "« " (title)
                }
            }
        }
    })
}

pub fn html_doc(
    page_title: &str,
    header_html: &str,
    article_inner: &str,
    footer_html: &str,
    catalog_html: &str,
) -> String {
    let doc_type = "<!DOCTYPE html>";
    let toc_html = catalog_html
        .is_empty()
        .not()
        .then(|| html!(nav id="toc" { (catalog_html) }))
        .unwrap_or_default();

    let body_inner = html!(div id="grid-wrapper" {
      article { (article_inner) (footer_html) }
      "\n\n"
      (toc_html)
    });

    let html = html!(html lang="en-US" {
        head {
            r#"
<meta http-equiv="Content-Type" content="text/html; charset=utf-8">
<meta name="viewport" content="width=device-width">"#
            (format!("<title>{page_title}</title>"))
            (html_import_meta())
            (html_css())
            (html_import_style())
            (html_import_fonts())
            (html_import_math())
        }
        body { (header_html) (body_inner) }
    });
    format!("{}\n{}", doc_type, html)
}

pub fn html_css() -> String {
    match config::inline_css() {
        true => html!(style { (html_main_style()) (html_typst_style()) }),
        false => {
            let base_url = config::base_url();
            format!(
                r#"<link rel="stylesheet" href="{}main.css">
<link rel="stylesheet" href="{}typst.css">"#,
                base_url, base_url
            )
        }
    }
}

pub fn html_import_meta() -> String {
    return config::CUSTOM_META_HTML.clone();
}

pub fn html_import_style() -> String {
    return config::CUSTOM_STYLE_HTML.clone();
}

pub fn html_import_fonts() -> String {
    return config::CUSTOM_FONTS_HTML.clone();
}

pub fn html_import_math() -> String {
    return config::CUSTOM_MATH_HTML.clone();
}

pub fn html_main_style() -> &'static str {
    return include_str!("include/main.css");
}

pub fn html_typst_style() -> &'static str {
    return include_str!("include/typst.css");
}
