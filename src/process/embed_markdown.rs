// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use super::processer::{url_action, Processer};
use std::collections::HashMap;

use crate::{
    compiler::{
        parser::parse_spanned_markdown,
        section::{EmbedContent, HTMLContent, LazyContent, LocalLink, SectionOption},
    },
    html_flake::html_link,
    recorder::{ParseRecorder, State},
    slug::to_slug,
};
use eyre::{eyre, WrapErr};
use pulldown_cmark::{Tag, TagEnd};

pub struct Embed;

impl Processer for Embed {
    fn start(&mut self, tag: &Tag<'_>, recorder: &mut ParseRecorder) {
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
                }
            }
            Tag::MetadataBlock(_kind) => {
                recorder.enter(State::Metadata);
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: &TagEnd, recorder: &mut ParseRecorder) -> Option<LazyContent> {
        if *tag == TagEnd::Link && recorder.state == State::Embed {
            let entry_url = recorder.data.get(0).map_or("", |s| s);
            let entry_url = crate::config::relativize(entry_url);

            let embed_text = recorder.data.get(1);
            let (section_option, inline_title) = parse_embed_text(embed_text);

            recorder.exit();
            return Some(LazyContent::Embed(EmbedContent {
                url: entry_url,
                title: inline_title,
                option: section_option,
            }));
        }

        if *tag == TagEnd::Link && recorder.state == State::LocalLink {
            let url = recorder
                .data
                .get(0)
                .map_or(String::new(), |s| s.to_string());
            let text = match recorder.data.len() > 1 {
                true => Some(recorder.data[1..].join("")),
                false => None,
            };
            recorder.exit();

            return Some(LazyContent::Local(LocalLink {
                slug: to_slug(&url),
                text,
            }));
        }

        if *tag == TagEnd::Link && recorder.state == State::ExternalLink {
            let url = recorder
                .data
                .get(0)
                .map_or(String::new(), |s| s.to_string());
            let text = (recorder.data.len() > 1)
                .then(|| recorder.data[1..].join(""))
                .unwrap_or_default();
            let title = (url == text)
                .then(|| format!("{}", url))
                .unwrap_or_else(|| format!("{} [{}]", text, url));
            recorder.exit();

            let html = html_link(&url, &title, &text, State::ExternalLink.strify());
            return Some(LazyContent::Plain(html));
        }

        recorder.exit();
        None
    }

    fn text(
        &self,
        s: &pulldown_cmark::CowStr<'_>,
        recorder: &mut ParseRecorder,
        metadata: &mut HashMap<String, HTMLContent>,
    ) -> eyre::Result<()> {
        if allow_inline(&recorder.state) {
            recorder.push(s.to_string()); // [1, 2, ...]: Text
            return Ok(());
        }

        if recorder.state == State::Metadata && s.trim().len() != 0 {
            parse_metadata(s, metadata, recorder).wrap_err("failed to parse metadata")?;
        }
        Ok(())
    }

    fn inline_math(
        &self,
        s: &pulldown_cmark::CowStr<'_>,
        recorder: &mut ParseRecorder,
    ) -> Option<std::string::String> {
        if allow_inline(&recorder.state) {
            recorder.push(format!("${}$", s)); // [1, 2, ...]: Text
        }
        None
    }

    fn code(&self, s: &pulldown_cmark::CowStr<'_>, recorder: &mut ParseRecorder) {
        if allow_inline(&recorder.state) {
            recorder.push(format!("<code>{}</code>", s));
        }
    }
}

fn allow_inline(state: &State) -> bool {
    *state == State::Embed || *state == State::LocalLink || *state == State::ExternalLink
}

/// It is known that the behavior differs between the two architectures
/// `(I)` `x86_64-pc-windows-msvc` and `(II)` `aarch64-unknown-linux-musl`.
/// `(I)` automatically splits the input by lines,
/// while `(II)` receives the entire multi-line string as a whole.
pub fn parse_metadata(
    s: &str,
    metadata: &mut HashMap<String, HTMLContent>,
    recorder: &mut ParseRecorder,
) -> eyre::Result<()> {
    let lines: Vec<&str> = s.split("\n").collect();
    for s in lines {
        if s.trim().len() != 0 {
            let pos = s
                .find(':')
                .ok_or_else(|| eyre!("expected metadata format `name: value`, found `{s}`"))?;
            let key = s[0..pos].trim();
            let val = s[pos + 1..].trim();

            let mut val = parse_spanned_markdown(val, &format!("{}:metadata", recorder.current))
                .wrap_err("failed to parse metadata value")?;
            if key == "taxon" {
                if let HTMLContent::Plain(v) = val {
                    val = HTMLContent::Plain(display_taxon(&v));
                }
            }
            metadata.insert(key.to_string(), val);
        }
    }
    Ok(())
}

pub fn parse_embed_text(embed_text: Option<&String>) -> (SectionOption, Option<String>) {
    match embed_text {
        None => (SectionOption::default(), None),
        Some(embed_text) => {
            let mut numbering = false;
            let mut details_open = true;
            let mut catalog = true;

            let mut index = 0;
            let chars = embed_text.chars();
            for curr in chars {
                match curr {
                    '+' => numbering = true,
                    '-' => details_open = false,
                    '.' => catalog = false,
                    _ => break,
                }
                index += 1;
            }

            let option = SectionOption::new(numbering, details_open, catalog);
            let inline_title = &embed_text[index..].trim();
            let inline_title = match !inline_title.is_empty() {
                true => Some(inline_title.to_string()),
                false => None,
            };
            (option, inline_title)
        }
    }
}

pub fn display_taxon(s: &str) -> String {
    match s.split_at_checked(1) {
        Some((first, rest)) => format!("{}. ", first.to_uppercase() + rest),
        _ => s.to_string(),
    }
}

/*
 * URI scheme:
 *   http, https, ftp, mailto, file, data and irc
 */
fn is_external_link(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://") || url.starts_with("www.")
}

fn is_local_link(url: &str) -> bool {
    !super::typst_image::is_inline_typst(url) && !is_external_link(url) && !url.contains(":")
}
