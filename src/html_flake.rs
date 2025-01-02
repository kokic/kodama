
use crate::{html, recorder::{Catalog, CatalogItem, Counter}};

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
    let open = match open {
        true => "open",
        false => ""
    };

    let inner_html = format!("{}{}", (html!(summary => {summary})), content);
    let html_details = format!(r#"
      <details id={} {}>{}</details>
    "#, id, open, inner_html);
    html!(section class = {class_name.join(" ")}, data_taxon = {taxon} => {html_details})
}

pub fn html_entry_header(
    author: &str,
    start_date: Option<&String>,
    end_date: Option<&String>,
    mut etc: Vec<String>,
) -> String {
    let mut meta_items: Vec<String> = vec![];
    if let Some(start_date) = start_date {
        meta_items.push(start_date.to_string());
    }
    if let Some(end_date) = end_date {
        meta_items.push(end_date.to_string());
    }
    meta_items.push(author.to_string());
    meta_items.append(&mut etc);

    let items = meta_items
        .iter()
        .map(|item| html!(li class = "meta-item" => {item}))
        .reduce(|s, t| s + &t)
        .unwrap_or(String::new());

    html!(div class="metadata" => (html!(ul => {items})))
}

pub fn html_image(image_src: &str) -> String {
    format!("<img src = \"{image_src}\" />")
}

pub fn html_center_image(image_src: &str) -> String {
    html!(div style = "text-align: center" => {html_image(image_src)})
}

pub fn html_link(href: &str, title: &str, text: &str, class_name: &str) -> String {
    html!(span class = format!("link {}", class_name) => 
      (html!(a href = {href}, title = {title} => {text})))
}

pub fn html_doc(article_inner: &str, catalog_html: &str) -> String {
    let doc_type = "<!DOCTYPE html>";
    let body_inner = html!(div id="grid-wrapper" => 
      (html!(article => {article_inner}))
      "\n\n"
      (html!(nav id = "toc" => {catalog_html})));

    let html = html!(html lang = "en-US" => 
      (html!(head => r#"
<meta http-equiv="Content-Type" content="text/html; charset=utf-8">
<meta name="viewport" content="width=device-width"> 
<title></title>"#
        (html_import_fonts())
        (html_import_katex())))
        (html_css())
        (html_javascript())
      (html!(body => {body_inner})));
    format!("{}\n{}", doc_type, &html)
}

fn html_toc_li(data: &CatalogItem, counter: &Counter) -> String {
    let (slug, taxon, text) = (data.slug.as_str(), data.taxon.as_str(), data.text.as_str());
    let slug_url = format!("{}.html", slug);
    let title = format!("{} [{}]", text, slug);
    let href = format!("#{}", slug); // #id

    let mut child_html = String::new();
    if !data.children.is_empty() {
        child_html.push_str(r#"<ul class="block">"#);
        let mut counter = counter.left_shift();
        for child in &data.children {
            child.number.then(|| counter.step_mut());
            child_html.push_str(&html_toc_li(&child, &counter));
        }
        child_html.push_str("</ul>");
    }

    let taxon = data.number.then(|| {
        let taxon_numbering = format!("{} {} ", taxon, counter.display());
        taxon_numbering
    }).unwrap_or(taxon.to_string());

    let mut class_name: Vec<String> = vec![];
    if data.summary {
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

pub fn html_toc_block(data: &Catalog) -> String {
    // let mut taxon_map: HashMap<String, String> = HashMap::new();
    let mut counter = Counter::init();
    let items = data
        .iter()
        .map(|item| {
            item.number.then(|| counter.step_mut());
            html_toc_li(item, &counter)
        })
        .reduce(|s, t| s + &t)
        .unwrap_or(String::new());
    let html_toc = html!(div class = "block" => 
      (html!(h1 => "Table of Contents"))
      (html!(ul class = "block" => {items})));
    html_toc
}

pub fn html_javascript() -> String {
    html!(script => 
      (include_str!("include/page-title.js"))
      (include_str!("include/section-taxon.js")))
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

pub fn html_main_style() -> &'static str {
    return include_str!("include/main.css");
}
