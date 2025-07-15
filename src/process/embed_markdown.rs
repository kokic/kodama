// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use super::{content::EventExtended, processer::url_action};
use std::mem;

use crate::{
    compiler::section::{EmbedContent, LocalLink, SectionOption},
    html_flake::html_link,
    recorder::State,
    slug::to_slug,
};
use pulldown_cmark::{html, Event, Tag, TagEnd};

pub struct Embed<'e, E> {
    events: E,
    state: State,
    url: Option<String>,
    content: Vec<Event<'e>>,
}

impl<'e, E> Embed<'e, E> {
    pub fn process(events: E) -> Self {
        Self {
            events,
            state: State::None,
            url: None,
            content: Vec::new(),
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

impl<'e, E: Iterator<Item = Event<'e>>> Iterator for Embed<'e, E> {
    type Item = EventExtended<'e>;

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
                        return Some(e.into());
                    }
                }
                Event::End(TagEnd::Link) => match self.state {
                    State::Embed => {
                        let (url, mut content) = self.exit();
                        let url = crate::config::relativize(&url);

                        let mut option = SectionOption::default();
                        let title = if let Some(e) = content.first_mut() {
                            // parse options, then strip /[-+.]/ from beginning of the title
                            if let Event::Text(t) = e {
                                let (opt, rest) = parse_embed_text(t);
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
                        return Some(EmbedContent { title, url, option }.into());
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
                        return Some(
                            LocalLink {
                                slug: to_slug(&url),
                                text,
                            }
                            .into(),
                        );
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
                        return Some(Event::Html(html.into()).into());
                    }
                    _ => return Some(e.into()),
                },
                Event::Text(_) if allow_inline(&self.state) => self.content.push(e),
                Event::InlineMath(ref math) => {
                    let replaced = Event::Text(format!("${math}$").into());
                    if allow_inline(&self.state) {
                        self.content.push(replaced);
                    } else {
                        return Some(replaced.into());
                    }
                }
                // TODO: move away from mangling math manually
                Event::DisplayMath(ref math) => {
                    return Some(Event::Text(format!("$${math}$$").into()).into())
                }
                Event::Code(_) if allow_inline(&self.state) => {
                    self.content.push(e);
                }
                _ => return Some(e.into()),
            }
        }
        None
    }
}

fn parse_embed_text(embed_text: &str) -> (SectionOption, String) {
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
