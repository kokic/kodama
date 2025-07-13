// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use super::{content::EventExtended, processer::url_action};
use std::{collections::HashMap, mem};

use crate::{
    compiler::{
        parser::parse_spanned_markdown2,
        section::{EmbedContent, HTMLContent, LocalLink, SectionOption},
    },
    html_flake::html_link,
    recorder::State,
    slug::{to_slug, Slug},
};
use eyre::{eyre, WrapErr};
use pulldown_cmark::{html, Event, Tag, TagEnd};

pub struct Embed2<'e, 'm, E> {
    events: E,
    state: State,
    url: Option<String>,
    content: Vec<Event<'e>>,
    metadata: &'m mut HashMap<String, HTMLContent>,
}

impl<'e, 'm, E> Embed2<'e, 'm, E> {
    pub fn new(events: E, metadata: &'m mut HashMap<String, HTMLContent>) -> Self {
        Self {
            events,
            state: State::None,
            url: None,
            content: Vec::new(),
            metadata,
        }
    }

    fn exit(&mut self) -> (String, Vec<Event<'e>>) {
        self.state = State::None;
        (
            self.url.take().unwrap_or_default(),
            mem::take(&mut self.content),
        )
    }
}

impl<'e, 'm, E: Iterator<Item = Event<'e>>> Iterator for Embed2<'e, 'm, E> {
    type Item = eyre::Result<EventExtended<'e>>;

    fn next(&mut self) -> Option<Self::Item> {
        for e in self.events.by_ref() {
            match e {
                Event::Start(Tag::Link { ref dest_url, .. }) => {
                    let (mut url, action) = url_action(dest_url);
                    if action == State::Embed.strify() {
                        self.state = State::Embed;
                        self.url = Some(url); // [0]
                    } else if is_external_link(&url) {
                        self.state = State::ExternalLink;
                        self.url = Some(url);
                    } else if is_local_link(dest_url) {
                        self.state = State::LocalLink;

                        if url.ends_with(".md") {
                            url.truncate(url.len() - 3);
                        }
                        self.url = Some(url);
                    } else {
                        return Some(Ok(e.into()));
                    }
                }
                Event::Start(Tag::MetadataBlock(_)) => {
                    self.state = State::Metadata;
                }
                Event::End(TagEnd::MetadataBlock(_)) => {
                    self.state = State::None;
                }
                Event::End(TagEnd::Link) => match self.state {
                    State::Embed => {
                        let (url, mut content) = self.exit();
                        let url = crate::config::relativize(&url);
                        let mut option = SectionOption::default();
                        let title = if let Some(e) = content.first_mut() {
                            // parse options, then strip /[-+.]/ from beginning of the title
                            if let Event::Text(t) = e {
                                let (opt, rest) = parse_embed_text2(t);
                                option = opt;
                                *t = rest.into();
                            }
                            let mut title = String::new();
                            html::push_html(&mut title, content.into_iter());
                            Some(title)
                        } else {
                            None
                        };
                        let title = title.filter(|t| !t.is_empty());
                        return Some(Ok(EmbedContent { title, url, option }.into()));
                    }
                    State::LocalLink => {
                        let (url, content) = self.exit();
                        let text = if content.is_empty() {
                            None
                        } else {
                            let mut text = String::new();
                            html::push_html(&mut text, content.into_iter());
                            Some(text)
                        };
                        return Some(Ok(LocalLink {
                            slug: to_slug(&url),
                            text,
                        }
                        .into()));
                    }
                    State::ExternalLink => {
                        let (url, content) = self.exit();
                        let mut text = String::new();
                        html::push_html(&mut text, content.into_iter());
                        let formatted_title;
                        let title = if url == text {
                            &url
                        } else {
                            formatted_title = format!("{text} [{url}]");
                            &formatted_title
                        };
                        let html = html_link(&url, title, &text, State::ExternalLink.strify());
                        return Some(Ok(Event::Html(html.into()).into()));
                    }
                    _ => return Some(Ok(e.into())),
                },
                Event::Text(ref text) => {
                    if allow_inline(&self.state) {
                        self.content.push(e);
                    } else if self.state == State::Metadata && !text.trim().is_empty() {
                        if let Err(e) = parse_metadata2(text, self.metadata) {
                            return Some(Err(e.wrap_err("failed to parse metadata")));
                        }
                    } else {
                        return Some(Ok(e.into()));
                    }
                }
                Event::InlineMath(ref math) => {
                    let replaced = Event::Text(format!("${math}$").into());
                    if allow_inline(&self.state) {
                        self.content.push(replaced);
                    } else {
                        return Some(Ok(replaced.into()));
                    }
                }
                // TODO: move away from mangling math manually
                Event::DisplayMath(ref math) => {
                    return Some(Ok(Event::Text(format!("$${math}$$").into()).into()))
                }
                Event::Code(_) if allow_inline(&self.state) => {
                    self.content.push(e);
                }
                _ => return Some(Ok(e.into())),
            }
        }
        None
    }
}

/// It is known that the behavior differs between the two architectures
/// `(I)` `x86_64-pc-windows-msvc` and `(II)` `aarch64-unknown-linux-musl`.
/// `(I)` automatically splits the input by lines,
/// while `(II)` receives the entire multi-line string as a whole.
pub fn parse_metadata2(s: &str, metadata: &mut HashMap<String, HTMLContent>) -> eyre::Result<()> {
    let lines: Vec<&str> = s.split("\n").collect();
    for s in lines {
        if !s.trim().is_empty() {
            let pos = s
                .find(':')
                .ok_or_else(|| eyre!("expected metadata format `name: value`, found `{s}`"))?;
            let key = s[0..pos].trim();
            let val = s[pos + 1..].trim();

            let res = parse_spanned_markdown2(val, Slug::new(metadata["slug"].as_str().unwrap()));
            let mut val = res.wrap_err("failed to parse metadata value")?;

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

pub fn parse_embed_text2(embed_text: &str) -> (SectionOption, String) {
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
    let inline_title = &embed_text[index..];
    (option, inline_title.to_owned())
}

fn allow_inline(state: &State) -> bool {
    *state == State::Embed || *state == State::LocalLink || *state == State::ExternalLink
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
