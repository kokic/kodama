// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use super::{
    content::EventExtended,
    processer::{url_action, Processer},
};
use std::{collections::HashMap, mem};

use crate::{
    compiler::{
        parser::{parse_spanned_markdown, parse_spanned_markdown2},
        section::{EmbedContent, HTMLContent, LazyContent, LocalLink, SectionOption},
    },
    html_flake::html_link,
    recorder::{ParseRecorder, State},
    slug::{to_slug, Slug},
};
use eyre::{eyre, WrapErr};
use pulldown_cmark::{html, Event, Tag, TagEnd};

pub struct Embed;

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

            let res = parse_spanned_markdown(val, &recorder.current);
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
