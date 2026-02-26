// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Alias Qli (@AliasQli)

use eyre::eyre;
use htmlize::unescape_attribute;
use regex_lite::{CaptureMatches, Captures, Regex};
use std::borrow::Cow;
use std::collections::HashMap;
use std::str;
use std::sync::LazyLock;

#[derive(Clone, Copy)]
pub enum HTMLTagKind {
    Meta,
    Embed,
    Subtree,
    Local { span: bool },
}

impl HTMLTagKind {
    fn new(name: &str, span: bool) -> Option<HTMLTagKind> {
        match name {
            "meta" => Some(HTMLTagKind::Meta),
            "embed" => Some(HTMLTagKind::Embed),
            "subtree" => Some(HTMLTagKind::Subtree),
            "local" => Some(HTMLTagKind::Local { span }),
            _ => None,
        }
    }

    fn tri_equal(&self, k: &HTMLTagKind) -> Option<bool> {
        match (self, k) {
            (HTMLTagKind::Meta, HTMLTagKind::Meta)
            | (HTMLTagKind::Embed, HTMLTagKind::Embed)
            | (HTMLTagKind::Subtree, HTMLTagKind::Subtree) => Some(true),
            (HTMLTagKind::Local { span: a }, HTMLTagKind::Local { span: b }) => {
                if a == b {
                    Some(true)
                } else {
                    None
                }
            }
            _ => Some(false),
        }
    }
}

struct HTMLTag {
    kind: HTMLTagKind,
    start: usize,
    end: usize,
    mid: Option<usize>,
}

pub struct HTMLMatch<'a> {
    pub kind: HTMLTagKind,
    pub start: usize,
    pub end: usize,
    pub attrs: HashMap<&'a str, Cow<'a, str>>,
    pub body: &'a str,
}

pub struct HTMLParser<'a> {
    html_str: &'a str,
    captures: CaptureMatches<'static, 'a>,
}

impl<'a> HTMLParser<'a> {
    pub fn new(html_str: &'a str) -> HTMLParser<'a> {
        static RE_TAG: LazyLock<Regex> = LazyLock::new(|| {
            fn real(alt: u8) -> String {
                format!(r#"?<real{}>"#, alt)
            }
            fn kodama(alt: u8) -> String {
                format!(r#"kodama-(?<tag{}>meta|embed|subtree|local)"#, alt)
            }
            fn local(alt: u8) -> String {
                format!(r#"kodama-(?<tag{}>local)"#, alt)
            }
            fn attrs(alt: u8) -> String {
                format!(
                    r#"(?<attrs{}>(\s+([a-zA-Z-]+)(="([^"\\]|\\[\s\S])*")?)*)"#,
                    alt
                )
            }
            Regex::new(&format!(
                r#"<span>\s*({}<{}{}>)|({}</{}>)\s*</span>|<{}{}>|</{}>"#,
                real(0),
                local(0),
                attrs(0),
                real(1),
                local(1),
                kodama(2),
                attrs(2),
                kodama(3),
            ))
            .unwrap()
        });
        HTMLParser {
            html_str,
            captures: RE_TAG.captures_iter(html_str),
        }
    }
}

impl<'a> Iterator for HTMLParser<'a> {
    type Item = eyre::Result<HTMLMatch<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        fn get_tag(capture: Captures<'_>) -> eyre::Result<(HTMLTag, Option<&str>)> {
            let all = capture
                .get(0)
                .ok_or_else(|| eyre!("missing full match while parsing typst html tag"))?;
            let make_tag = |kind, mid| HTMLTag {
                start: all.start(),
                end: all.end(),
                mid,
                kind,
            };
            if let Some(name) = capture.name("tag0") {
                Ok((
                    make_tag(
                        HTMLTagKind::new(name.as_str(), true)
                            .ok_or_else(|| eyre!("unknown kodama tag `{}`", name.as_str()))?,
                        Some(
                            capture
                                .name("real0")
                                .ok_or_else(|| eyre!("missing `real0` capture"))?
                                .start(),
                        ),
                    ),
                    Some(
                        capture
                            .name("attrs0")
                            .ok_or_else(|| eyre!("missing `attrs0` capture"))?
                            .as_str(),
                    ),
                ))
            } else if let Some(name) = capture.name("tag1") {
                Ok((
                    make_tag(
                        HTMLTagKind::new(name.as_str(), true)
                            .ok_or_else(|| eyre!("unknown kodama tag `{}`", name.as_str()))?,
                        Some(
                            capture
                                .name("real1")
                                .ok_or_else(|| eyre!("missing `real1` capture"))?
                                .end(),
                        ),
                    ),
                    None,
                ))
            } else if let Some(name) = capture.name("tag2") {
                Ok((
                    make_tag(
                        HTMLTagKind::new(name.as_str(), false)
                            .ok_or_else(|| eyre!("unknown kodama tag `{}`", name.as_str()))?,
                        None,
                    ),
                    Some(
                        capture
                            .name("attrs2")
                            .ok_or_else(|| eyre!("missing `attrs2` capture"))?
                            .as_str(),
                    ),
                ))
            } else if let Some(name) = capture.name("tag3") {
                Ok((
                    make_tag(
                        HTMLTagKind::new(name.as_str(), false)
                            .ok_or_else(|| eyre!("unknown kodama tag `{}`", name.as_str()))?,
                        None,
                    ),
                    None,
                ))
            } else {
                Err(eyre!("unexpected tag capture while parsing typst html"))
            }
        }

        let mut stack = vec![];

        let (mut open_tag, mattrs) = match self.captures.next() {
            Some(capture) => match get_tag(capture) {
                Ok(tag) => tag,
                Err(err) => return Some(Err(err)),
            },
            None => return None,
        };
        let attrs_str = match mattrs {
            Some(attrs) => attrs,
            None => return Some(Err(eyre!("expecting open kodama tag, found closing tag"))),
        };
        stack.push(open_tag.kind);

        let mut close_tag = loop {
            let capture = match self.captures.next() {
                Some(capture) => capture,
                None => return Some(Err(eyre!("unclosed kodama tag: unexpected end of html"))),
            };
            let (tag, mattrs) = match get_tag(capture) {
                Ok(tag) => tag,
                Err(err) => return Some(Err(err)),
            };

            if mattrs.is_some() {
                stack.push(tag.kind);
            } else {
                let last = match stack.pop() {
                    Some(tag) => tag,
                    None => return Some(Err(eyre!("found closing tag without matching open tag"))),
                };
                if tag.kind.tri_equal(&last) == Some(false) {
                    return Some(Err(eyre!("mismatched nested kodama tags")));
                }
                if stack.is_empty() {
                    break tag;
                }
            }
        };

        if open_tag.kind.tri_equal(&close_tag.kind) != Some(true) {
            open_tag.mid.inspect(|mid| open_tag.start = *mid);
            close_tag.mid.inspect(|mid| close_tag.end = *mid);
        }

        static RE_ATTR: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"(?<key>[a-zA-Z-]+)(="(?<value>([^"\\]|\\[\s\S])*)")?"#).unwrap()
        });

        let mut attrs: HashMap<&str, Cow<'_, str>> = HashMap::new();
        for c in RE_ATTR.captures_iter(attrs_str) {
            let key = match c.name("key") {
                Some(key) => key.as_str(),
                None => return Some(Err(eyre!("malformed attribute in typst html tag"))),
            };
            attrs.insert(
                key,
                unescape_attribute(c.name("value").map_or("", |s| s.as_str())),
            );
        }

        Some(Ok(HTMLMatch {
            kind: open_tag.kind,
            start: open_tag.start,
            end: close_tag.end,
            attrs,
            body: self.html_str[open_tag.end..close_tag.start].trim(),
        }))
    }
}
