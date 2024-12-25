use crate::html;

pub fn html_section(
    summary: &String,
    content: &String,
    hide_metadata: bool,
    taxon: Option<&String>,
) -> String {
    let mut class_name: Vec<&str> = vec!["block"];
    if hide_metadata {
        class_name.push("hide-metadata");
    }
    let taxon = taxon.map(|s| s.as_str()).unwrap_or("entry");
    html!(section class = {class_name.join(" ")}, data_taxon = {taxon} =>
      (html!(details id = "#id", open = "true" =>
        (html!(summary => {summary}))
        (content)))
    )
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

pub fn html_css() -> &'static str {
    return r###"
<meta http-equiv="Content-Type" content="text/html; charset=utf-8">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="true">
<link href="https://fonts.googleapis.com/css2?family=Source+Code+Pro:ital,wght@0,200..900;1,200..900&amp;family=Source+Sans+3:ital,wght@0,200..900;1,200..900&amp;family=Source+Serif+4:ital,opsz,wght@0,8..60,200..900;1,8..60,200..900&amp;display=swap" rel="stylesheet">
<meta name="viewport" content="width=device-width">
<link rel="stylesheet" href="/main.css">
"###;
}

pub fn html_import_katex() -> &'static str {
    return r###"
<link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/katex@0.16.15/dist/katex.min.css" integrity="sha384-Htz9HMhiwV8GuQ28Xr9pEs1B4qJiYu/nYLLwlDklR53QibDfmQzi7rYxXhMH/5/u" crossorigin="anonymous">
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.15/dist/katex.min.js" integrity="sha384-bxmi2jLGCvnsEqMuYLKE/KsVCxV3PqmKeK6Y6+lmNXBry6+luFkEOsmp5vD9I/7+" crossorigin="anonymous"></script>
<script defer src="https://cdn.jsdelivr.net/npm/katex@0.16.15/dist/contrib/auto-render.min.js" integrity="sha384-hCXGrW6PitJEwbkoStFjeJxv+fSOOQKOPbJxSfM6G5sWZjAyWhXiTIIAmQqnlLlh" crossorigin="anonymous"></script>
<script>
  document.addEventListener("DOMContentLoaded", function() {
      renderMathInElement(document.body, {
        delimiters: [
            {left: '$$', right: '$$', display: true},
            {left: '$', right: '$', display: false},
            {left: '\\(', right: '\\)', display: false},
            {left: '\\[', right: '\\]', display: true}
        ],
        throwOnError : false
      });
  });
</script>
"###;
}

pub fn html_image(image_src: &str) -> String {
    format!("<img src = \"{image_src}\" />")
}

pub fn html_center_image(image_src: &str) -> String {
    html!(div style = "text-align: center" => {html_image(image_src)})
}

pub fn html_doc(article_inner: &str, catalog: &str) -> String {
    let doc_type = "<!DOCTYPE html>";
    let body_inner = html!(div id="grid-wrapper" => 
      (html!(article => {article_inner}))
      "\n\n"
      (html!(nav id = "toc" => {catalog})));

    let html = html!(html lang = "en" => 
      (html!(head => 
        (html_css()) 
        (html_import_katex())))
      (html!(body => {body_inner})));
    format!("{}\n{}", doc_type, &html)
}

/// `data: (slug: String, text: String)`
fn html_toc_li(data: &(String, String)) -> String {
    let (slug, text) = data;
    let slug_url = format!("{}.html", slug);
    let title = format!("{} {}", text, slug);
    html!(li => 
      (html!(a class = "bullet", href={slug_url}, title={title} => "■"))
      (html!(span class = "link local" => {text})))
}

/// `data: Vec<(slug: String, text: String)>`
pub fn html_toc_block(data: &Vec<(String, String)>) -> String {
    let items = data
        .iter()
        .map(html_toc_li)
        .reduce(|s, t| s + &t)
        .unwrap_or(String::new());
    html!(div class = "block" => 
      (html!(h1 => "Table of Contents"))
      (html!(ul class = "block" => {items})))
}
