use crate::{html, recorder::{Catalog, CatalogItem}};

pub fn html_section(
    summary: &String,
    content: &String,
    hide_metadata: bool,
    id: String, 
    taxon: Option<&String>,
) -> String {
    let mut class_name: Vec<&str> = vec!["block"];
    if hide_metadata {
        class_name.push("hide-metadata");
    }
    let taxon = taxon.map(|s| s.as_str()).unwrap_or("entry");
    html!(section class = {class_name.join(" ")}, data_taxon = {taxon} =>
      (html!(details id = {id}, open = "true" =>
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

pub fn html_image(image_src: &str) -> String {
    format!("<img src = \"{image_src}\" />")
}

pub fn html_center_image(image_src: &str) -> String {
    html!(div style = "text-align: center" => {html_image(image_src)})
}

pub fn html_link_local(href: &str, title: &str, text: &str) -> String {
    html!(span class = "link local" => 
      (html!(a href = {href}, title = {title} => {text})))
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

fn html_toc_li(data: &CatalogItem) -> String {
    let (slug, text) = (data.slug.as_str(), data.text.as_str());
    let slug_url = format!("{}.html", slug);
    let title = format!("{} [{}]", text, slug);
    let href = format!("#{}", slug);

    let mut child_html = String::new();
    if !data.children.is_empty() {
        child_html.push_str("<ul>");
        for child in &data.children {
            child_html.push_str(&html_toc_li(&child));
        }
        child_html.push_str("</ul>");
    }

    html!(li => 
      (html!(a class = "bullet", href={slug_url}, title={title} => "■"))
      (html!(span class = "link" => 
        (html!(a href = {href} => {text})))) 
      (child_html))
}

pub fn html_toc_block(data: &Catalog) -> String {
    let items = data
        .iter()
        .map(|item| html_toc_li(item))
        .reduce(|s, t| s + &t)
        .unwrap_or(String::new());
    html!(div class = "block" => 
      (html!(h1 => "Table of Contents"))
      (html!(ul class = "block" => {items})))
}

pub fn html_css() -> String {
    return format!(
        r###"
<meta http-equiv="Content-Type" content="text/html; charset=utf-8">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="true">
<link href="https://fonts.googleapis.com/css2?family=Source+Code+Pro:ital,wght@0,200..900;1,200..900&amp;family=Source+Sans+3:ital,wght@0,200..900;1,200..900&amp;family=Source+Serif+4:ital,opsz,wght@0,8..60,200..900;1,8..60,200..900&amp;display=swap" rel="stylesheet">
<meta name="viewport" content="width=device-width">
<style>
{}
</style>
"###,
        html_main_style()
    );
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
        throwOnError : false, 
        minRuleThickness: 0.05, 
      });
  });
</script>
"###;
}

pub fn html_main_style() -> &'static str {
    return r###"
      body {
        font-family: "Source Serif 4", serif;
        font-optical-sizing: auto;
        hyphens: auto;
      }
      
      p, pre {
        line-height: 1.55;
      }
      
      h1, h2, h3, h4 {
        margin-top: .5em;
      }
      
      h1, h2, h3, h4, h5, h6 {
        font-weight: normal;
        font-family: "Source Sans 3", sans-serif;
        font-weight: 500;
        margin-bottom: 0;
      }
      
      h5, h6, p {
        margin-top: 0;
      }
      
      details>summary {
        list-style-type: none;
        outline: none;
      }
      
      details>summary>header {
        display: inline;
      }
      
      /* no effect */
      details>summary::marker,
      details>summary::-webkit-details-marker {
        display: none;
      }
      
      details h1 {
        font-size: 1.2em;
        display: inline;
      }
      
      details>summary {
        list-style-type: none;
      }
            
      section .block[data-taxon] details>summary>header>h1 {
        font-size: 13pt;
      }
      
      article>section>details>summary>header>h1 {
        font-size: 1.5em;
      }
      
      article>section>details>summary>header {
        display: block;
        margin-bottom: .5em;
      }

      article>section>details>summary>header>h1>.taxon {
        display: block;
        font-size: .9em;
        color: #888;
        padding-bottom: 5pt;
      }
      
      /* class */
      .inline-typst {
        display: inline-block;
        margin: 0 0;
        line-height: 1em;
        vertical-align: middle;
      }
      
      .block {
        padding-left: 5px;
        padding-right: 10px;
        padding-bottom: 2px;
        border-radius: 5px;
      }
      
      .block:hover {
        background-color: rgba(0, 100, 255, 0.04);
      }
      
      .block.hide-metadata>details>summary>header>.metadata {
        display: none;
      }
      
      .metadata ul {
        padding-left: 0;
        display: inline;
      }
      
      .metadata li::after {
        content: " · ";
      }
      
      .metadata li:last-child::after {
        content: "";
      }
      
      .metadata ul li {
        display: inline;
      }
      
      a.link.local,
      .link.local a,
      a.slug {
        box-shadow: none;
        text-decoration-line: underline;
        text-decoration-style: dotted;
      }

      a {
        color: black;
        text-decoration: underline;
      }
      
      .slug,
      .doi,
      .orcid {
        color: gray;
        font-weight: 200;
      }
      
      #grid-wrapper>article {
        max-width: 90ex;
        margin-right: auto;
        grid-column: 1;
      }
      
      #grid-wrapper>nav {
        grid-column: 2;
      }
      
      @media only screen and (max-width: 1000px) {
        body {
          margin-top: 1em;
          margin-left: .5em;
          margin-right: .5em;
          transition: ease all .2s;
        }
      
        #grid-wrapper>nav {
          display: none;
          transition: ease all .2s;
        }
      }
      
      @media only screen and (min-width: 1000px) {
        body {
          margin-top: 2em;
          margin-left: 2em;
          transition: ease all .2s;
        }
      
        #grid-wrapper {
          display: grid;
          grid-template-columns: 90ex;
        }
      }
      
      nav#toc ul {
        list-style-type: none;
      }
      nav#toc, nav#toc a {
        color: #555;
      }
      
      nav {
        font-family: "Source Sans 3", sans-serif;
        font-optical-sizing: auto;
      }
      
      nav#toc a.bullet {
        opacity: 0.7;
        margin-left: 0.4em;
        margin-right: 0.3em;
        padding-left: 0.2em;
        padding-right: 0.2em;
        text-decoration: none;
      }      
    "###;
}
