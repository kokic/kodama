// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use super::{content::EventExtended, processer::url_action};
use std::{fs, mem};

use crate::{
    compiler::section::{EmbedContent, LocalLink, SectionOption},
    environment::{self, assets_dir, root_dir},
    html_flake::{html_code_block, html_link},
    process::typst_image::is_inline_typst,
    recorder::State,
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
                        self.url = Some(relocate_trees_path(url)); // [0]
                    } else if action == State::Include.strify() {
                        self.state = State::Include;
                        // Note: `Include` path starts from the root directory
                        self.url = Some(url);
                    } else if is_external_link(&url) {
                        self.state = State::ExternalLink;
                        self.url = Some(url);
                    } else if is_local_link(dest_url) {
                        self.state = State::LocalLink;

                        if url.ends_with(".md") {
                            url.truncate(url.len() - 3);
                        }
                        self.url = Some(relocate_trees_path(url));
                    } else if is_assets_file(&url) {
                        self.state = State::AssetFile;
                        self.url = Some(url);
                    } else {
                        return Some(e.into());
                    }
                }
                Event::End(TagEnd::Link) => match self.state {
                    State::Embed => {
                        let (url, mut content) = self.exit();

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
                    State::Include => {
                        let (url, content) = self.exit();
                        let language_tag = if content.is_empty() {
                            Some("plain".to_string())
                        } else {
                            let mut text = String::new();
                            html::push_html(&mut text, content.into_iter());
                            Some(text)
                        };

                        let content = fs::read_to_string(root_dir().join(&url))
                            .unwrap_or_else(|_| format!("failed to include file: {url}"));
                        let escaped = htmlize::escape_text(content);
                        let html = html_code_block(&escaped, &language_tag.unwrap_or_default());
                        return Some(Event::Html(html.into()).into());
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
                        return Some(LocalLink { url, text }.into());
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
                    State::AssetFile => {
                        let (url, content) = self.exit();
                        let mut text = String::new();
                        html::push_html(&mut text, content.into_iter());
                        let html = html_link(&url, &text, &text, State::AssetFile.strify());
                        return Some(Event::Html(html.into()).into());
                    }
                    _ => return Some(e.into()),
                },
                Event::Text(_) if is_inline_allowed(&self.state) => self.content.push(e),
                Event::InlineMath(ref math) => {
                    let replaced = Event::Text(format!("${math}$").into());
                    if is_inline_allowed(&self.state) {
                        self.content.push(replaced);
                    } else {
                        return Some(replaced.into());
                    }
                }
                // TODO: move away from mangling math manually
                Event::DisplayMath(ref math) => {
                    return Some(Event::Text(format!("$${math}$$").into()).into())
                }
                Event::Code(_) if is_inline_allowed(&self.state) => {
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

/// Returns `true` if the current state allows inline elements such as `Text`, `Code`, and `InlineMath` to be included in the content buffer.
fn is_inline_allowed(state: &State) -> bool {
    *state == State::Embed
        || *state == State::Include
        || *state == State::LocalLink
        || *state == State::ExternalLink
        || *state == State::AssetFile
}

pub fn display_taxon(s: &str) -> String {
    match s.split_at_checked(1) {
        Some((first, rest)) => format!("{}. ", first.to_uppercase() + rest),
        _ => s.to_string(),
    }
}

/// Relocate the path `/<trees>/path` to `/path`
fn relocate_trees_path(path: String) -> String {
    let trees = environment::trees_dir_without_root();
    let trees = format!("/{}", trees);
    if path.starts_with(&trees) {
        return path[trees.len()..].to_string();
    }
    path
}

/// URI scheme: http, https, ftp, mailto, file, data and irc
fn is_external_link(url: &str) -> bool {
    url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("www.")
        || url.starts_with("ftp://")
        || url.starts_with("mailto:")
        || url.starts_with("file://")
        || url.starts_with("data:")
        || url.starts_with("irc://")
}

/// Returns `true` if the URL represents a static asset file in the configured assets directory (check via [`assets_dir`]).
fn is_assets_file(url: &str) -> bool {
    let assets_dir = assets_dir();
    let assets_dir_str = assets_dir.as_str(); // to "./<assets_dir>"
    std::path::Path::new(&format!(".{}", url)).starts_with(assets_dir_str)
        || std::path::Path::new(&format!("./{}", url)).starts_with(assets_dir_str)
}

/// Returns `true` if the URL represents a local wiki link.  
///  
/// A URL is considered a local link if it satisfies all of the following:  
/// - Does not end with `/` (not a directory reference)  
/// - Is not inline Typst syntax (checked via [`is_inline_typst`])  
/// - Is not an external link (no `http://`, `https://`, or `www.` prefix, checked via  [`is_external_link`])  
/// - Contains no `:` character (no URI scheme or special action syntax, e.g., `#:embed`, checked via [`url_action`])  
/// - Does not start with the configured assets directory path  (e.g., `assets`, checked via [`assets_dir`]), as this is reserved for static assets
///  
/// Local links are processed into `LocalLink` events during markdown parsing,  
/// with `.md` extensions automatically stripped.  
fn is_local_link(url: &str) -> bool {
    !url.ends_with("/")
        && !is_inline_typst(url)
        && !is_external_link(url)
        && !is_assets_file(url)
        && !url.contains(":")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_assets_file() {
        crate::environment::mock_environment().unwrap();

        assert!(is_assets_file("assets/image.png"));
        assert!(is_assets_file("/assets/image.png"));
        assert!(is_assets_file("\\assets\\image.png"));

        assert!(!is_assets_file("image.png"));
        assert!(!is_assets_file("path/to/assets/image.png"));
        assert!(!is_assets_file("/path/to/image.png"));
    }

    #[test]
    fn test_is_local_link() {
        crate::environment::mock_environment().unwrap();

        assert!(is_local_link("./0AB7"));
        assert!(is_local_link("./0AB7.md"));
        assert!(is_local_link("/path/to/0AB7"));

        assert!(!is_local_link("http://example.com"));
        assert!(!is_local_link("https://example.com"));
        assert!(!is_local_link("www.example.com"));
        assert!(!is_local_link("external:page"));
        assert!(!is_local_link("inline"));
        assert!(!is_local_link("inline-0pt-0pt"));
        assert!(!is_local_link("assets/image.png"));
        assert!(!is_local_link("/assets/image.png"));
        assert!(!is_local_link("local-dir/"));
    }

    #[test]
    fn test_relocate_trees_path() {
        crate::environment::mock_environment().unwrap();

        assert_eq!(
            relocate_trees_path("/path".to_string()),
            "/path".to_string()
        );
        assert_eq!(
            relocate_trees_path("/trees/path".to_string()),
            "/path".to_string()
        );
    }
}
