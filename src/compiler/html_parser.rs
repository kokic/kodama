use htmlize::unescape_attribute;
use lazy_static::lazy_static;
use regex_lite::{CaptureMatches, Captures, Regex};
use std::borrow::Cow;
use std::collections::HashMap;
use std::str;

#[derive(PartialEq, Clone, Copy)]
pub enum HTMLTagKind {
    Meta,
    Embed,
    Local { span: bool },
}

impl HTMLTagKind {
    fn new(name: &str, span: bool) -> HTMLTagKind {
        match name {
            "meta" => HTMLTagKind::Meta,
            "embed" => HTMLTagKind::Embed,
            "local" => HTMLTagKind::Local { span },
            _ => unreachable!(),
        }
    }
}

struct HTMLTag {
    kind: HTMLTagKind,
    start: usize,
    end: usize,
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
        lazy_static! {
            static ref re_tag: Regex = {
                fn kodama(alt: u8) -> String {
                    format!(r#"kodama(?<tag{}>meta|embed|local)"#, alt)
                }
                fn local(alt: u8) -> String {
                    format!(r#"kodama(?<tag{}>local)"#, alt)
                }
                fn attrs(alt: u8) -> String {
                    format!(r#"(?<attrs{}>(\s+([a-zA-Z-]+)="([^"\\]|\\[\s\S])*")*)"#, alt)
                }
                Regex::new(&format!(
                    r#"<span>\s*<{}{}>|</{}>\s*</span>|<{}{}>|</{}>"#,
                    local(0),
                    attrs(0),
                    local(1),
                    kodama(2),
                    attrs(2),
                    kodama(3),
                ))
                .unwrap()
            };
        }
        HTMLParser {
            html_str,
            captures: re_tag.captures_iter(&html_str),
        }
    }
}

impl<'a> Iterator for HTMLParser<'a> {
    type Item = HTMLMatch<'a>;

    // HTML is typst-generated, so it's not expected to be ill-formatted.
    // Using panics here.
    fn next(&mut self) -> Option<Self::Item> {
        fn get_tag<'a>(capture: Captures<'a>) -> (HTMLTag, Option<&'a str>) {
            let all = capture.get(0).unwrap();
            let make_tag = |kind| HTMLTag {
                start: all.start(),
                end: all.end(),
                kind,
            };
            if let Some(name) = capture.name("tag0") {
                (
                    make_tag(HTMLTagKind::new(name.as_str(), true)),
                    Some(capture.name("attrs0").unwrap().as_str()),
                )
            } else if let Some(name) = capture.name("tag1") {
                (make_tag(HTMLTagKind::new(name.as_str(), true)), None)
            } else if let Some(name) = capture.name("tag2") {
                (
                    make_tag(HTMLTagKind::new(name.as_str(), false)),
                    Some(capture.name("attrs2").unwrap().as_str()),
                )
            } else if let Some(name) = capture.name("tag3") {
                (make_tag(HTMLTagKind::new(name.as_str(), false)), None)
            } else {
                unreachable!()
            }
        }

        let mut stack = vec![];

        let (open_tag, mattrs) = match self.captures.next() {
            Some(capture) => get_tag(capture),
            None => return None,
        };
        let attrs_str = mattrs.expect("Expecting an open tag, found closed tag");
        stack.push(open_tag.kind);

        let close_tag = loop {
            let capture = self.captures.next().expect("Expect more kodama tags");
            let (tag, mattrs) = get_tag(capture);

            if mattrs.is_some() {
                stack.push(tag.kind);
            } else {
                let last = stack.pop().unwrap();
                if tag.kind != last {
                    panic!("Tags don't match")
                }
                if stack.is_empty() {
                    break tag;
                }
            }
        };

        lazy_static! {
            static ref re_attr: Regex =
                Regex::new(r#"(?<key>[a-zA-Z]+)="(?<value>([^"\\]|\\[\s\S])*)""#).unwrap();
        }

        let attrs: HashMap<&str, Cow<'_, str>> = re_attr
            .captures_iter(attrs_str)
            .map(|c| {
                (
                    c.name("key").unwrap().as_str(),
                    unescape_attribute(c.name("value").unwrap().as_str()).to_owned(),
                )
            })
            .collect();

        Some(HTMLMatch {
            kind: open_tag.kind,
            start: open_tag.start,
            end: close_tag.end,
            attrs,
            body: &self.html_str[open_tag.end..close_tag.start].trim(),
        })
    }
}
