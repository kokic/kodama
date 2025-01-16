use std::{collections::HashMap, path::Path};

use super::{url_action, Handler};
use crate::{
    config::{self, verify_and_update_content_hash, Blink},
    entry::HtmlEntry,
    html_flake::{html_doc, html_link, html_toc_block},
    kodama::{compile_to_html, html_article_inner},
    recorder::{CatalogItem, Recorder, State},
};
use pulldown_cmark::{Tag, TagEnd};

pub struct Embed;

impl Handler for Embed {
    fn start(&mut self, tag: &Tag<'_>, recorder: &mut Recorder) {
        match tag {
            Tag::Link {
                link_type: _,
                dest_url,
                title: _,
                id: _,
            } => {
                let (mut url, action) = url_action(dest_url);
                if action == State::Embed.strify() {
                    recorder.enter(State::Embed);
                    recorder.push(url); // [0]
                } else if is_external_link(&url) {
                    recorder.enter(State::ExternalLink);
                    recorder.push(url);
                } else if is_local_link(&dest_url) {
                    recorder.enter(State::LocalLink);

                    if url.ends_with(".md") {
                        url.truncate(url.len() - 3);
                    }
                    recorder.push(url.to_string());

                    let mut linked = config::LINKED.lock().unwrap();
                    /*
                     * The reason why the `url` can be directly processed like this is that
                     * the link format used by the user must be an absolute path relative to
                     * the entire workspace, that is, a URL in the form of "/path/to/file".
                     *
                     * Finally, the prefix and suffix like `"./{}.html"` are used to
                     * maintain a consistent format when comparing with `history`.
                     */
                    linked.push(Blink::new(
                        recorder.current.to_string(),
                        format!(".{}.md", url),
                    ));
                }
            }
            Tag::MetadataBlock(_kind) => {
                recorder.enter(State::Metadata);
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: &TagEnd, recorder: &mut Recorder) -> Option<String> {
        if *tag == TagEnd::Link && recorder.state == State::Embed {
            let entry_url = recorder.data.get(0).unwrap().as_str();
            let entry_url = crate::config::relativize(entry_url);

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
                let mut hide_in_catalog = false;

                let chars = inline_title.chars();
                let mut index = 0;
                for curr in chars {
                    match curr {
                        '+' => use_numbering = true,
                        '-' => open_section = false,
                        '.' => hide_in_catalog = true,
                        _ => break,
                    }
                    index += 1;
                }
                inline_title = &inline_title[index..].trim();
                if inline_title.is_empty() {
                    inline_title = title;
                }

                // for catalog taxon
                let item: CatalogItem = CatalogItem {
                    slug: slug.to_string(),
                    text: inline_title.to_string(),
                    taxon: html_entry.metadata.taxon().unwrap().to_string(),
                    number: use_numbering,
                    summary: !open_section,
                    hide: hide_in_catalog,
                    children: html_entry.catalog.clone(),
                };
                recorder.catalog.push(Box::new(item));

                (inline_title.to_string(), open_section)
            };

            let mut inline_article = |html_entry: &mut HtmlEntry| {
                // generate inline article
                let (title, open) = update_catalog(&html_entry);
                html_entry.update("title".to_string(), title);
                let inline_article = html_article_inner(&html_entry, true, open);
                inline_article
            };

            let file_path = entry_url;
            match compile_to_html(&file_path) {
                Ok(mut html_entry) => {
                    let inline_article = inline_article(&mut html_entry);
                    recorder.exit();
                    return Some(inline_article);
                }
                Err(err) => eprintln!("{:?} at {}", err, recorder.current),
            }
        }

        if *tag == TagEnd::Link && recorder.state == State::LocalLink {
            let url = recorder.data.get(0).unwrap().to_string();
            let text = match recorder.data.len() > 1 {
                true => recorder.data[1..].join(""),
                false => url.to_string(),
            };
            recorder.exit();
            return Some(html_link(
                &config::full_url(&url),
                &text,
                &text,
                State::LocalLink.strify(),
            ));
        }

        if *tag == TagEnd::Link && recorder.state == State::ExternalLink {
            let url = recorder.data.get(0).unwrap().to_string();
            let text = match recorder.data.len() > 1 {
                true => recorder.data[1..].join(""),
                false => url.to_string(),
            };
            recorder.exit();
            return Some(html_link(&url, &text, &text, State::ExternalLink.strify()));
        }

        match tag {
            TagEnd::MetadataBlock(_kind) => recorder.exit(),
            _ => {}
        }
        None
    }

    fn text(
        &self,
        s: &pulldown_cmark::CowStr<'_>,
        recorder: &mut Recorder,
        metadata: &mut HashMap<String, String>,
    ) {
        if recorder.state == State::Embed
            || recorder.state == State::LocalLink
            || recorder.state == State::ExternalLink
        {
            return recorder.push(s.to_string()); // [1, 2, ...]: Text
        }

        if recorder.state == State::Metadata && s.trim().len() != 0 {
            /*
             * It is known that the behavior differs between the two architectures
             * (I) `x86_64-pc-windows-msvc` and (II) `aarch64-unknown-linux-musl`.
             * (I) automatically splits the input by lines,
             * while (II) receives the entire multi-line string as a whole.
             */
            let lines: Vec<&str> = s.split("\n").collect();
            for s in lines {
                if s.trim().len() != 0 {
                    let pos = s.find(':').expect("metadata item expect `name: value`");
                    let key = s[0..pos].trim();
                    let val = s[pos + 1..].trim();
                    metadata.insert(key.to_string(), val.to_string());
                }
            }
        }
    }
}

pub fn update_taxon(html_entry: &mut HtmlEntry) {
    let taxon = display_option_taxon(html_entry.metadata.taxon());
    html_entry.update("taxon".to_string(), taxon);
}

pub fn write_to_html(html_url: &str, entry: &mut HtmlEntry) {
    update_taxon(entry);
    let catalog_html = html_toc_block(&entry.catalog);
    let article_inner = html_article_inner(entry, false, true);
    let html = html_doc(&article_inner, &catalog_html);

    let filepath = crate::config::output_path(&html_url);
    if verify_and_update_content_hash(&filepath, &html) {
        let _ = std::fs::write(&filepath, html);
        println!(
            "Output: {:?} {}",
            entry.metadata.title().map_or("", |s| s),
            crate::slug::pretty_path(Path::new(&filepath))
        );
    }
}

pub fn display_taxon(s: &str) -> String {
    match s.split_at_checked(1) {
        Some((first, rest)) => format!("{}. ", first.to_uppercase() + rest),
        _ => s.to_string(),
    }
}

pub fn display_option_taxon(taxon: Option<&String>) -> String {
    match taxon {
        None => "".to_string(),
        Some(s) => display_taxon(s),
    }
}

fn is_external_link(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://") || url.starts_with("www.")
}

fn is_local_link(url: &str) -> bool {
    !super::typst_image::is_inline_typst(url) && !is_external_link(url) && !url.contains("#:")
}
