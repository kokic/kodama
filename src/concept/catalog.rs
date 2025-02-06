// use std::collections::HashMap;

// use serde::{Deserialize, Serialize};

// use crate::{html, recorder::Counter};

// use super::taxon::Taxon;

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct CatalogItem {
//     pub slug: String,
//     pub text: String,
//     pub taxon: Taxon,
//     pub numbering: bool,
//     pub summary: bool,
//     pub hide: bool,
//     pub children: Catalog,
// }

// pub type Catalog = Vec<Box<CatalogItem>>;

// // pub fn update_numbering_taxon(
// //     catalog: &mut Catalog,
// //     counter: &mut Counter,
// //     branch_slug: &str,
// //     // lookup: &mut HashMap<String, String>,
// // ) {
// //     catalog.iter_mut().for_each(|item| {
// //         if item.numbering {
// //             counter.step_mut();
// //             item.taxon.numbering = Some(counter.display());
            
// //             let numbered_taxon = item.taxon.display();
// //             let global_slug = format!("{}{}", branch_slug, item.slug);
// //             // lookup.insert(global_slug.to_string(), numbered_taxon);

// //             if !item.children.is_empty() {
// //                 let mut subcounter = counter.left_shift();
// //                 update_numbering_taxon(&mut item.children, &mut subcounter, &global_slug, lookup);
// //             }
// //         }
// //     });
// // }

// pub fn html_toc_block(catalog: &Catalog) -> String {
//     if catalog.is_empty() {
//         return String::new();
//     }

//     let items = catalog
//         .iter()
//         .map(html_toc_li)
//         .reduce(|s, t| s + &t)
//         .unwrap_or(String::new());

//     let html_toc = html!(div class = "block" =>
//       (html!(h1 => "Table of Contents"))
//       (html!(ul class = "block" => {items})));
//     html_toc
// }

// fn html_toc_li(data: &Box<CatalogItem>) -> String {
//     let (slug, taxon, text) = (data.slug.as_str(), data.taxon.display(), data.text.as_str());
//     let slug_url = format!("{}{}", slug, crate::config::page_suffix());
//     let title = format!("{} [{}]", text, slug);
//     let href = format!("#{}", crate::slug::to_hash_id(slug)); // #id

//     let mut child_html = String::new();
//     if !data.children.is_empty() {
//         child_html.push_str(r#"<ul class="block">"#);
//         for child in &data.children {
//             child_html.push_str(&html_toc_li(&child));
//         }
//         child_html.push_str("</ul>");
//     }

//     let mut class_name: Vec<String> = vec![];
//     if data.summary {
//         class_name.push("item-summary".to_string());
//     }
//     // if data.hide {
//     //     class_name.push("display-none".to_string());
//     // }

//     html!(li class = {class_name.join(" ")} =>
//       (html!(a class = "bullet", href={slug_url}, title={title} => "■"))
//       (html!(span class = "link" =>
//         (html!(a href = {href} =>
//           (html!(span class = "taxon" => {taxon}))
//           (text)))))
//       (child_html))
// }
