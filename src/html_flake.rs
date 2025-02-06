
use std::ops::Not;

use crate::html;

pub fn html_section(
    summary: &String,
    content: &String,
    hide_metadata: bool,
    open: bool,  
    id: String, 
    taxon: Option<&String>,
) -> String {
    let mut class_name: Vec<&str> = vec!["block"];
    if hide_metadata {
        class_name.push("hide-metadata");
    }
    let taxon = taxon.map_or("", |s| s);
    let open = open.then(|| "open").unwrap_or("");
    let inner_html = format!("{}{}", (html!(summary => {summary})), content);
    let html_details = format!(r#"
      <details id="{}" {}>{}</details>
    "#, id, open, inner_html);
    html!(section class = {class_name.join(" ")}, data_taxon = {taxon} => {html_details})
}

pub fn html_entry_header(
    mut etc: Vec<String>,
) -> String {
    let mut meta_items: Vec<String> = vec![];
    meta_items.append(&mut etc);

    let items = meta_items
        .iter()
        .map(|item| html!(li class = "meta-item" => {item}))
        .reduce(|s, t| s + &t)
        .unwrap_or(String::new());

    html!(div class="metadata" => (html!(ul => {items})))
}

pub fn html_image(image_src: &str) -> String {
    format!(r#"<img src = "{image_src}" />"#)
}

pub fn html_figure(image_src: &str, center: bool, caption: String) -> String {
    if !center {
        return html_image(image_src);
    }
    let mut caption = caption;
    if !caption.is_empty() {
        caption = html!(figcaption => (caption))
    }
    html!(figure => (html_image(image_src)) (caption))
}

pub fn html_link(href: &str, title: &str, text: &str, class_name: &str) -> String {
    html!(span class = format!("link {}", class_name) => 
      (html!(a href = {href}, title = {title} => {text})))
}

pub fn html_doc(page_title: &str, article_inner: &str, catalog_html: &str) -> String {
    let doc_type = "<!DOCTYPE html>";
    let toc_html = catalog_html
        .is_empty()
        .not()
        .then(|| html!(nav id = "toc" => {catalog_html}))
        .unwrap_or_default();
    
    let body_inner = html!(div id="grid-wrapper" => 
      (html!(article => {article_inner}))
      "\n\n"
      (toc_html));

    let html = html!(html lang = "en-US" => 
      (html!(head => r#"
<meta http-equiv="Content-Type" content="text/html; charset=utf-8">
<meta name="viewport" content="width=device-width">"#
        (format!("<title>{page_title}</title>"))
        (html_import_fonts())
        (html_import_katex())
        (html_auto_render())))
        (html_css())
      (html!(body => {body_inner})));
    format!("{}\n{}", doc_type, &html)
}

pub fn html_css() -> String {
    html!(style => 
      (html_main_style()))
}

pub fn html_import_fonts() -> &'static str {
    return include_str!("include/import-fonts.html");
}

pub fn html_import_katex() -> &'static str {
    return include_str!("include/import-katex.html");
}

pub fn html_auto_render() -> &'static str {
    return include_str!("include/auto-render.html");
}

pub fn html_main_style() -> &'static str {
    return include_str!("include/main.css");
}
