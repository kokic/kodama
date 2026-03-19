// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use crate::{cli::serve, environment, html_macro::html};

const MOBILE_TOC_SCRIPT: &str = include_str!("../include/mobile-toc.js");
const THEME_SCRIPT: &str = include_str!("../include/theme.js");
const MAIN_STYLE: &str = include_str!("../include/main.css");

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
        include_str!("../include/reload.html").to_string()
    } else {
        String::new()
    }
}

pub fn html_scripts() -> String {
    let template = html_theme_option_template();

    if environment::inline_script() {
        return format!(
            "{template}<script>\n{MOBILE_TOC_SCRIPT}\n</script><script>\n{THEME_SCRIPT}\n</script>"
        );
    }

    let base_url = environment::base_url();
    format!(
        r#"{template}<script src="{base_url}mobile-toc.js"></script><script src="{base_url}theme.js"></script>"#
    )
}

fn html_theme_option_template() -> String {
    html!(template id="theme-option-template" {
        r#"<input type="radio" name="theme" /><label></label>"#
    })
}

pub fn html_mobile_toc_script() -> &'static str {
    MOBILE_TOC_SCRIPT
}

pub fn html_theme_script() -> &'static str {
    THEME_SCRIPT
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
    MAIN_STYLE
}
