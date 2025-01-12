use std::{collections::HashMap, path::Path};

use super::{url_action, Handler};
use crate::{
    config::{self, verify_and_update_content_hash},
    entry::HtmlEntry,
    html_flake::{html_doc, html_link, html_toc_block},
    kodama::{adjust_name, compile_to_html, html_article_inner, parse_markdown},
    recorder::{CatalogItem, Context, Recorder},
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
                let (url, action) = url_action(dest_url);
                if action == Context::Embed.strify() {
                    recorder.enter(Context::Embed);
                    recorder.push(url); // [0]
                } else if is_external_link(&url) {
                    recorder.enter(Context::ExternalLink);
                    recorder.push(url);
                } else if is_local_link(&dest_url) {
                    recorder.enter(Context::LocalLink);
                    recorder.push(url.to_string());

                    let filename = format!(".{}.md", &url);
                    compile_to_html(&filename);
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
            let entry_url = crate::config::relativize(entry_url);
            // let entry_url = config::join_path(&recorder.relative_dir, entry_url);
            // let (parent_dir, filename) = crate::config::parent_dir(&entry_url);

            // url & path
            let file_path = entry_url;
            let mut html_url = adjust_name(&file_path, ".md", ".html");
            // let file_path = config::join_path(&parent_dir, &html_url);
            html_url = crate::config::output_path(&html_url);

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

            let inline_article = |html_entry: &mut HtmlEntry| {
                let taxon = display_option_taxon(html_entry.metadata.taxon());
                html_entry.update("taxon".to_string(), taxon);
                write_to_html(&html_url, html_entry);
                let mut history = config::history();
                history.push(html_url);

                // generate inline article
                let (title, open) = update_catalog(&html_entry);
                html_entry.update("title".to_string(), title);
                let inline_article = html_article_inner(&html_entry, true, open);

                inline_article
            };

            let mut html_entry = parse_markdown(&file_path);
            let inline_article = inline_article(&mut html_entry);
            recorder.exit();
            return Some(inline_article);
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
            return Some(html_link(
                &config::full_url(&url),
                &text,
                &text,
                Context::LocalLink.strify(),
            ));
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

    fn text(
        &self,
        s: &pulldown_cmark::CowStr<'_>,
        recorder: &mut Recorder,
        metadata: &mut HashMap<String, String>,
    ) {
        if recorder.context == Context::Embed
            || recorder.context == Context::LocalLink
            || recorder.context == Context::ExternalLink
        {
            return recorder.push(s.to_string()); // [1]: Text
        }

        if recorder.context == Context::Metadata && s.trim().len() != 0 {
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

pub fn write_to_html(filepath: &str, entry: &HtmlEntry) {
    let catalog_html = html_toc_block(&entry.catalog);
    let article_inner = html_article_inner(entry, false, true);
    let html = html_doc(&article_inner, &catalog_html);

    let history = config::history();
    let key = filepath.to_string();

    if !history.contains(&key) && verify_and_update_content_hash(&filepath, &html) {
        let _ = std::fs::write(filepath, html);
        println!(
            "Output: {:?} {}",
            entry.metadata.title().map_or("", |s| s),
            crate::slug::pretty_path(Path::new(filepath))
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
