// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::collections::HashMap;

use crate::{
    compiler::{parser::parse_spanned_markdown, section::HTMLContent},
    slug::Slug,
};
use eyre::eyre;
use pulldown_cmark::{Event, Tag, TagEnd};

pub struct Metadata<'m, E> {
    events: E,
    state: bool,
    metadata: &'m mut HashMap<String, HTMLContent>,
}

impl<'m, E> Metadata<'m, E> {
    pub fn new(events: E, metadata: &'m mut HashMap<String, HTMLContent>) -> Self {
        Self {
            events,
            state: false,
            metadata,
        }
    }
}

impl<'e, 'm, E: Iterator<Item = Event<'e>>> Iterator for Metadata<'m, E> {
    type Item = eyre::Result<Event<'e>>;

    fn next(&mut self) -> Option<Self::Item> {
        for e in self.events.by_ref() {
            match e {
                Event::Start(Tag::MetadataBlock(_)) => {
                    self.state = true;
                }
                Event::End(TagEnd::MetadataBlock(_)) => {
                    self.state = false;
                }
                Event::Text(ref text) => {
                    if self.state && !text.trim().is_empty() {
                        if let Err(e) = parse_metadata(text, self.metadata) {
                            return Some(Err(e.wrap_err("failed to parse metadata")));
                        }
                    } else {
                        return Some(Ok(e));
                    }
                }
                _ => return Some(Ok(e)),
            }
        }
        None
    }
}

/// It is known that the behavior differs between the two architectures
/// `(I)` `x86_64-pc-windows-msvc` and `(II)` `aarch64-unknown-linux-musl`.
/// `(I)` automatically splits the input by lines,
/// while `(II)` receives the entire multi-line string as a whole.
fn parse_metadata(s: &str, metadata: &mut HashMap<String, HTMLContent>) -> eyre::Result<()> {
    let lines: Vec<&str> = s.split("\n").collect();
    for s in lines {
        if !s.trim().is_empty() {
            let pos = s
                .find(':')
                .ok_or_else(|| eyre!("expected metadata format `name: value`, found `{s}`"))?;
            let key = s[0..pos].trim();
            let val = s[pos + 1..].trim();

            let res = parse_spanned_markdown(val, Slug::new(metadata["slug"].as_str().unwrap()));
            let mut val = res;

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

fn display_taxon(s: &str) -> String {
    match s.split_at_checked(1) {
        Some((first, rest)) => format!("{}. ", first.to_uppercase() + rest),
        _ => s.to_string(),
    }
}
