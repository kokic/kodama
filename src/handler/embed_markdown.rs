use super::{url_action, Handler};
use crate::{
    adjust_name, config::{self, entry_path}, entry::HtmlEntry, html_article_inner, html_flake::{html_doc, html_link, html_toc_block}, parse_markdown, recorder::{CatalogItem, Context, Recorder}, ParseInterrupt
};
use pulldown_cmark::{Tag, TagEnd};

pub struct Embed {}

impl Handler for Embed {
    fn start(&mut self, tag: &Tag<'_>, recorder: &mut Recorder) {
        match tag {
            Tag::Link {
                link_type: _,
                dest_url,
                title: _,
                id: _,
            } => {
                let (url, action) = url_action(dest_url);
                if action == Context::Embed.strify() {
                    recorder.enter(Context::Embed);
                    recorder.push(url.to_string()); // [0]
                } else if is_external_link(&url) {
                    recorder.enter(Context::ExternalLink);
                    recorder.push(url.to_string());
                } else {
                    recorder.enter(Context::LocalLink);
                    recorder.push(url.to_string());
                }
            }
            Tag::MetadataBlock(_kind) => {
                recorder.enter(Context::Metadata);
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: &TagEnd, recorder: &mut Recorder) -> Option<String> {
        if *tag == TagEnd::Link && recorder.context == Context::Embed {
            let entry_url = recorder.data.get(0).unwrap().as_str();
            let entry_url = config::join_path(&recorder.relative_dir, entry_url);
            let (parent_dir, filename) = crate::config::parent_dir(&entry_url);

            // url & path
            let mut html_url = adjust_name(&filename, ".md", ".html");
            let file_path = config::join_path(&parent_dir, &html_url);
            html_url = crate::config::output_path(&file_path);

            let mut update_catalog = |html_entry: &HtmlEntry| {
                let slug = html_entry.get("slug").map_or("[no_slug]", |s| s);
                let title = html_entry.metadata.title().map_or("[no_title]", |s| s);

                let mut inline_title = recorder
                    .data
                    .get(1) // inline entry title
                    .map(|s| s.as_str())
                    .unwrap_or(title);

                let mut use_numbering = false;
                let mut open_section = true;

                let chars = inline_title.chars();
                let mut index = 0;
                for curr in chars {
                    match curr {
                        '+' => use_numbering = true,
                        '-' => open_section = false,
                        _ => break,
                    }
                    index += 1;
                }
                inline_title = &inline_title[index..].trim();
                if inline_title.is_empty() {
                    inline_title = title;
                }

                // for catalog taxon
                let taxon = display_taxon(html_entry.metadata.taxon());
                let item: CatalogItem = CatalogItem {
                    slug: slug.to_string(),
                    text: inline_title.to_string(),
                    taxon: taxon.to_string(),
                    number: use_numbering,
                    summary: !open_section,
                    children: html_entry.catalog.clone(),
                };
                recorder.catalog.push(Box::new(item));

                (inline_title.to_string(), taxon.to_string(), open_section)
            };

            match parse_markdown(&parent_dir, &filename) {
                Ok(mut html_entry) => {
                    // cache .entry file
                    let entry_path = entry_path(&format!("{}.entry", file_path));
                    let _ = std::fs::write(entry_path, serde_json::to_string(&html_entry).unwrap());

                    write_entry_html(&html_url, &mut html_entry);

                    // generate inline article
                    let (title, taxon, open) = update_catalog(&html_entry);
                    html_entry.update("taxon".to_string(), taxon);
                    html_entry.update("title".to_string(), title);
                    let inline_article = html_article_inner(&html_entry, true, open);

                    recorder.exit();
                    return Some(inline_article);
                }
                Err(kind @ ParseInterrupt::Skiped) => {
                    // reuse .entry file
                    let entry_path = entry_path(&format!("{}.entry", file_path));
                    let serialized =
                        std::fs::read_to_string(entry_path).expect(config::ERR_ENTRY_FILE_LOST);
                    let mut html_entry: HtmlEntry =
                        serde_json::from_str(&serialized).expect(config::ERR_INVALID_ENTRY_FILE);

                    // generate inline article
                    let (title, taxon, open) = update_catalog(&html_entry);
                    html_entry.update("taxon".to_string(), taxon);
                    html_entry.update("title".to_string(), title);
                    let inline_article = html_article_inner(&html_entry, true, open);

                    println!("{}", kind.message(Some(&file_path)));
                    recorder.exit();
                    return Some(inline_article);
                }
                Err(kind) => eprintln!("{}", kind.message(Some(&file_path))),
            }
        }

        if *tag == TagEnd::Link && recorder.context == Context::LocalLink {
            let url = recorder.data.get(0).unwrap().to_string();
            let text = recorder
                .data
                .get(1)
                .map(|s| s.as_str())
                .unwrap_or(url.as_str())
                .to_string();
            recorder.exit();
            return Some(html_link(&url, &text, &text, Context::LocalLink.strify()));
        }

        if *tag == TagEnd::Link && recorder.context == Context::ExternalLink {
            let url = recorder.data.get(0).unwrap().to_string();
            let text = recorder
                .data
                .get(1)
                .map(|s| s.as_str())
                .unwrap_or(url.as_str())
                .to_string();
            recorder.exit();
            return Some(html_link(
                &url,
                &text,
                &text,
                Context::ExternalLink.strify(),
            ));
        }

        match tag {
            TagEnd::MetadataBlock(_kind) => recorder.exit(),
            _ => {}
        }
        None
    }

    fn text(&self, s: &pulldown_cmark::CowStr<'_>, recorder: &mut Recorder) {
        if recorder.context == Context::Embed
            || recorder.context == Context::LocalLink
            || recorder.context == Context::ExternalLink
        {
            recorder.push(s.to_string()); // [1]: Text
        }
    }
}

/// to cache entry file
pub fn write_entry_html(filepath: &str, entry: &HtmlEntry) {
    let catalog_html = html_toc_block(&entry.catalog);
    let article_inner = html_article_inner(entry, false, true);
    let html = html_doc(&article_inner, &catalog_html);
    let _ = std::fs::write(filepath, html);
}

pub fn display_taxon(taxon: Option<&String>) -> String {
    match taxon {
        None => "".to_string(),
        Some(s) => match s.split_at_checked(1) {
            Some((first, rest)) => format!("{}. ", first.to_uppercase() + rest),
            _ => s.to_string(),
        },
    }
}

fn is_external_link(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://") || url.starts_with("www.")
}

#[allow(dead_code)]
fn is_local_link(url: &str) -> bool {
    !is_external_link(url)
}
