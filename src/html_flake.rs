// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use crate::{
    cli::serve,
    entry::{EntryMetaData, MetaData},
    environment::{self, input_path},
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
    Ok(crate::html_flake::html_section(
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

pub fn html_header_metadata(mut etc: Vec<String>) -> String {
    let mut meta_items: Vec<String> = vec![];
    meta_items.append(&mut etc);
    let mut items = String::new();
    for item in &meta_items {
        items.push_str(&html!(li class="meta-item" { (item) }));
    }

    html!(div class="metadata" { ul { (items) } })
}

pub fn html_header(
    title: &str,
    taxon: &str,
    slug: &Slug,
    ext: &str,
    show_slug: bool,
    source_slug: Option<&str>,
    source_pos: Option<&str>,
    span_class: String,
    etc: Vec<String>,
) -> String {
    let slug_str = slug.as_str();
    let source_slug = source_slug.unwrap_or(slug_str);
    let is_serve = environment::is_serve();
    let serve_edit = environment::editor_url();
    let deploy_edit = environment::deploy_edit_url();

    let slug_text = EntryMetaData::to_slug_text(slug_str);
    let slug_url = environment::full_html_url(*slug);
    let slug_link = if show_slug {
        html!(a class="slug" href={slug_url} { "["(slug_text)"]" })
    } else {
        String::new()
    };

    let edit_text = environment::get_edit_text();
    let edit_class = "edit";
    let edit_url = match (is_serve, serve_edit, deploy_edit) {
        (true, Some(prefix), _) => {
            let source_path = input_path(format!("{}.{}", source_slug, ext));
            let editor_url = (|| {
                let source_path = source_path.canonicalize().ok()?;
                let source_url = url::Url::from_file_path(source_path).ok()?;
                let base = url::Url::parse(&prefix).ok()?;
                let url = base.join(source_url.path()).ok()?.to_string();
                Some(append_editor_position(url, &prefix, source_pos))
            })();

            match editor_url {
                Some(url) => html!(a class=edit_class href={url} { (edit_text) }),
                None => {
                    color_print::ceprintln!(
                        "<y>Warning: failed to construct editor URL for `{}` (source `{}`).</>",
                        slug,
                        source_slug
                    );
                    String::new()
                }
            }
        }
        (false, _, Some(prefix)) => {
            let source_path = format!("{}.{}", source_slug, ext);
            let editor_url =
                append_editor_position(format!("{}{}", prefix, source_path), &prefix, source_pos);
            html!(a class=edit_class href={editor_url.to_string()} { (edit_text) })
        }
        _ => String::default(),
    };

    html!(header {
        h1 {
            span class={span_class} { (taxon) }
            (title) " "
            (slug_link)
            (edit_url)
        }
        (html_header_metadata(etc))
    })
}

fn append_editor_position(url: String, prefix: &str, source_pos: Option<&str>) -> String {
    if !is_vscode_family_file_url(prefix) {
        return url;
    }
    let Some(pos) = source_pos else {
        return url;
    };
    if parse_source_pos(pos).is_none() {
        return url;
    }
    format!("{url}:{pos}")
}

fn is_vscode_family_file_url(prefix: &str) -> bool {
    [
        "vscode://file",
        "vscode-insiders://file",
        "vsc://file",
        "vscodium://file",
    ]
    .iter()
    .any(|candidate| prefix.starts_with(candidate))
}

fn parse_source_pos(pos: &str) -> Option<(usize, usize)> {
    let (line, col) = pos.split_once(':')?;
    let line = line.parse::<usize>().ok()?;
    let col = col.parse::<usize>().ok()?;
    if line == 0 || col == 0 {
        return None;
    }
    Some((line, col))
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
    let escaped_href = htmlize::escape_attribute(href);
    let escaped_title = htmlize::escape_attribute(title);
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
    use super::{append_editor_position, html_link, parse_source_pos};
    use crate::slug::Slug;

    #[test]
    fn test_html_link_escapes_title_attribute() {
        let html = html_link(
            "/AC2C",
            r#"<span lang="zh">abc</span> [AC2C]""#,
            r#"<span lang="zh">abc</span>"#,
            "local",
        );
        assert!(html.contains(r#"href="/AC2C""#));
        assert!(html.contains("title=\""));
        assert!(html.contains("&lt;span"));
        assert!(html.contains("&lt;/span&gt;"));
        assert!(!html.contains(r#"title="<span lang="zh">"#));
        assert!(html.contains(r#"><span lang="zh">abc</span></a>"#));
    }

    #[test]
    fn test_append_editor_position_for_vscode() {
        let url = append_editor_position(
            "vscode://file/c:/repo/docs/trees/book/index.md".to_string(),
            "vscode://file/",
            Some("12:3"),
        );
        assert_eq!(url, "vscode://file/c:/repo/docs/trees/book/index.md:12:3");
    }

    #[test]
    fn test_append_editor_position_for_vscode_family() {
        let cases = [
            (
                "vscode-insiders://file/c:/repo/docs/trees/book/index.md",
                "vscode-insiders://file/",
            ),
            ("vsc://file/c:/repo/docs/trees/book/index.md", "vsc://file/"),
            (
                "vscodium://file/c:/repo/docs/trees/book/index.md",
                "vscodium://file/",
            ),
        ];

        for (url, prefix) in cases {
            let with_pos = append_editor_position(url.to_string(), prefix, Some("12:3"));
            assert_eq!(with_pos, format!("{url}:12:3"));
        }
    }

    #[test]
    fn test_append_editor_position_ignores_non_vscode_family_or_invalid_pos() {
        let web = append_editor_position(
            "https://example.com/edit/path".to_string(),
            "https://example.com/edit/",
            Some("12:3"),
        );
        assert_eq!(web, "https://example.com/edit/path");

        let invalid = append_editor_position(
            "vscode://file/c:/repo/docs/trees/book/index.md".to_string(),
            "vscode://file/",
            Some("0:3"),
        );
        assert_eq!(invalid, "vscode://file/c:/repo/docs/trees/book/index.md");
    }

    #[test]
    fn test_parse_source_pos() {
        assert_eq!(parse_source_pos("1:1"), Some((1, 1)));
        assert_eq!(parse_source_pos("12:3"), Some((12, 3)));
        assert_eq!(parse_source_pos("0:3"), None);
        assert_eq!(parse_source_pos("abc"), None);
    }

    #[test]
    fn test_html_header_can_hide_slug_link() {
        let html = super::html_header(
            "Title",
            "Taxon. ",
            &Slug::new("book/child"),
            "md",
            false,
            None,
            None,
            "taxon".to_string(),
            Vec::new(),
        );
        assert!(!html.contains("class=\"slug\""));
    }
}

pub fn html_doc(
    page_title: &str,
    header_html: &str,
    article_inner: &str,
    footer_html: &str,
    catalog_html: &str,
) -> String {
    let mut toc_class: Vec<&str> = vec![];
    if environment::is_toc_sticky() {
        toc_class.push("sticky-nav");
    }
    if environment::is_toc_mobile_sticky() {
        toc_class.push("mobile-sticky-nav");
    }

    let base_url = environment::base_url();
    let doc_type = "<!DOCTYPE html>";

    let nav_html = html_nav(toc_class, catalog_html);
    let html = html!(html lang="en-US" {
        head {
            r#"
<meta http-equiv="Content-Type" content="text/html; charset=utf-8">
<meta name="viewport" content="width=device-width">"#
            (format!("<title>{page_title}</title>"))
            (format!(r#"<link rel="icon" href="{}assets/favicon.ico" />"#, base_url))
            (html_import_meta())
            (html_scripts())
            (html_live_reload())
            // math should be loaded after scripts to handle dynamic content
            (html_import_math())
            // main styles should be loaded after math to override formula font size
            (html_static_css())
            (html_dynamic_css())
            // fonts should be loaded after `static_css` to handle override default fonts
            (html_import_fonts())
            // custom styles should be loaded last to override other styles
            (html_import_style())
        }
        body {
            (header_html)
            (html_body_inner(&nav_html, article_inner, footer_html))
        }
    });
    format!("{}\n{}", doc_type, html)
}

fn html_body_inner(nav: &str, article_inner: &str, footer: &str) -> String {
    let base_url = environment::base_url_raw();
    let style = grid_wrapper_style();

    html!(div id="grid-wrapper" style={style} data_base_url={base_url} {
        (nav) "\n\n" article { (article_inner) (footer) }
    })
}

pub fn grid_wrapper_style() -> &'static str {
    if environment::is_toc_left() {
        "grid-template-areas: 'toc article';"
    } else {
        "grid-template-areas: 'article toc';"
    }
}

pub fn html_static_css() -> String {
    if environment::inline_css() {
        html!(style { (html_main_style()) })
    } else {
        let base_url = environment::base_url();
        format!(r#"<link rel="stylesheet" href="{}main.css">"#, base_url)
    }
}

pub fn html_dynamic_css() -> String {
    let toc_max_width = environment::toc_max_width();
    let grid_columns_value = if environment::is_toc_left() {
        "max-content var(--article-max-width)"
    } else {
        "var(--article-max-width) var(--toc-max-width)"
    };

    let grid_wrapper = format!(
        r#"@media only screen and (min-width: 1000px) {{
  #grid-wrapper {{ grid-template-columns: {grid_columns_value}; }}
  nav#toc {{ max-width: {toc_max_width}; }}
}}"#
    );

    format!("<style>\n{grid_wrapper}\n</style>")
}

pub fn html_import_meta() -> String {
    environment::import_meta_html()
}

pub fn html_import_style() -> String {
    environment::import_style_html()
}

pub fn html_import_fonts() -> String {
    environment::import_fonts_html()
}

pub fn html_import_math() -> String {
    environment::import_math_html()
}

pub fn html_live_reload() -> String {
    if *serve::live_reload() {
        include_str!("include/reload.html").to_string()
    } else {
        String::new()
    }
}

pub fn html_scripts() -> &'static str {
    concat!(
        include_str!("include/mobile-toc.html"),
        include_str!("include/theme.html"),
    )
}

fn html_import_theme() -> String {
    environment::theme_paths()
        .iter()
        .map(|theme_path| match std::fs::read_to_string(theme_path) {
            Ok(content) => content,
            Err(err) => {
                color_print::ceprintln!(
                    "<y>Warning: Failed to read theme file at '{}': {}</>",
                    theme_path,
                    err
                );

                String::new()
            }
        })
        .collect()
}

fn html_themes() -> String {
    html!(div id="theme-options" { (html_import_theme()) })
}

pub fn html_nav(toc_class: Vec<&str>, catalog_html: &str) -> String {
    html!(nav id="toc" class={toc_class.join(" ")} {
        (html_themes()) (catalog_html)
    })
}

pub fn html_main_style() -> &'static str {
    include_str!("include/main.css")
}
