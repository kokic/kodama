use std::ops::Not;

use crate::{compiler::taxon::Taxon, config, entry::EntryMetaData, html};

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
        metadata.taxon(),
    )
}

pub fn html_footer_section(content: &String) -> String {
    let summary = "<header><h1>References</h1></header>";
    let inner_html = format!("{}{}", (html!(summary => {summary})), content);
    let html_details = format!("<details open>{}</details>", inner_html);
    html!(footer => (html!(section class="block" => {html_details})))
}

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
    let data_taxon = Taxon::to_data_taxon(&taxon);
    let open = open.then(|| "open").unwrap_or("");
    let inner_html = format!("{}{}", (html!(summary => {summary})), content);
    let html_details = format!(
        r#"
      <details id="{}" {}>{}</details>
    "#,
        id, open, inner_html
    );
    html!(section class = {class_name.join(" ")}, data_taxon = {data_taxon} => {html_details})
}

pub fn html_entry_header(mut etc: Vec<String>) -> String {
    let mut meta_items: Vec<String> = vec![];
    meta_items.append(&mut etc);

    let items = meta_items
        .iter()
        .map(|item| html!(li class = "meta-item" => {item}))
        .reduce(|s, t| s + &t)
        .unwrap_or(String::new());

    html!(div class="metadata" => (html!(ul => {items})))
}

pub fn catalog_item(
    slug: &str,
    text: &str,
    details_open: bool,
    taxon: &str,
    child_html: &str,
) -> String {
    let slug_url = config::full_html_url(slug);
    let title = format!("{} [{}]", text, slug);
    let href = format!("#{}", crate::slug::to_hash_id(slug)); // #id

    let mut class_name: Vec<String> = vec![];
    if !details_open {
        class_name.push("item-summary".to_string());
    }

    html!(li class = {class_name.join(" ")} =>
      (html!(a class = "bullet", href={slug_url}, title={title} => "■"))
      (html!(span class = "link" =>
        (html!(a href = {href} =>
          (html!(span class = "taxon" => {taxon}))
          (text)))))
      (child_html))
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

pub fn html_header_nav(title: &str, href: &str) -> String {
    let nav_inner = html!(div class = "logo" => 
      (html!(a href={href}, title={title} => ("« ") (title))));

    html!(header class = "header" => 
      (html!(nav class = "nav" => {nav_inner})))
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
        .then(|| html!(nav id = "toc" => {catalog_html}))
        .unwrap_or_default();

    let body_inner = html!(div id="grid-wrapper" => 
      (html!(article => (article_inner) (footer_html)))
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
      (html!(body => (header_html) (body_inner))));
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
