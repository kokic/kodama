// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use crate::{
    compiler::{parser::parse_spanned_markdown, section::HTMLContent},
    entry::{is_plain_metadata, KEY_SLUG, KEY_TAXON},
    ordered_map::OrderedMap,
    slug::Slug,
};
use eyre::eyre;
use pulldown_cmark::{Event, Tag, TagEnd};

pub struct Metadata<'m, E> {
    events: E,
    state: bool,
    metadata: &'m mut OrderedMap<String, HTMLContent>,
}

impl<'m, E> Metadata<'m, E> {
    pub fn process(events: E, metadata: &'m mut OrderedMap<String, HTMLContent>) -> Self {
        Self {
            events,
            state: false,
            metadata,
        }
    }
}

impl<'e, E: Iterator<Item = Event<'e>>> Iterator for Metadata<'_, E> {
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
fn parse_metadata(s: &str, metadata: &mut OrderedMap<String, HTMLContent>) -> eyre::Result<()> {
    let current_slug = metadata
        .get(KEY_SLUG)
        .and_then(HTMLContent::as_str)
        .map(Slug::new)
        .ok_or_else(|| eyre!("missing `slug` while parsing metadata block"))?;

    for (line_no, s) in s.lines().enumerate() {
        if !s.trim().is_empty() {
            let pos = s.find(':').ok_or_else(|| {
                eyre!(
                    "invalid metadata in `{}` at line {}: expected `name: value`, found `{}`",
                    current_slug,
                    line_no + 1,
                    s
                )
            })?;
            let key = s[0..pos].trim();
            let val = s[pos + 1..].trim();

            let parsed = parse_metadata_value(key, val, current_slug);
            metadata.insert(key.to_string(), parsed);
        }
    }
    Ok(())
}

fn parse_metadata_value(key: &str, value: &str, current_slug: Slug) -> HTMLContent {
    if is_plain_metadata(key) {
        return HTMLContent::Plain(value.to_string());
    }

    let mut parsed = parse_spanned_markdown(value, current_slug);
    if key == KEY_TAXON {
        if let HTMLContent::Plain(v) = parsed {
            parsed = HTMLContent::Plain(display_taxon(&v));
        }
    }
    parsed
}

/// Format the taxon string for display.
pub fn display_taxon(s: &str) -> String {
    // Capitalize the first letter and add a period and space at the end.
    match s.split_at_checked(1) {
        Some((first, rest)) => format!("{}. ", first.to_uppercase() + rest),
        _ => format!("{}. ", s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{KEY_PAGE_TITLE, KEY_TITLE};

    fn metadata_with_slug(slug: &str) -> OrderedMap<String, HTMLContent> {
        let mut metadata = OrderedMap::new();
        metadata.insert(KEY_SLUG.to_string(), HTMLContent::Plain(slug.to_string()));
        metadata
    }

    #[test]
    fn test_page_title_is_plain_text_and_not_elaborated() {
        crate::environment::mock_environment().unwrap();

        let mut metadata = metadata_with_slug("index");
        parse_metadata("page-title: 中文", &mut metadata).unwrap();

        let parsed = metadata
            .get(KEY_PAGE_TITLE)
            .and_then(HTMLContent::as_str)
            .unwrap_or_default()
            .to_string();
        assert_eq!(parsed, "中文");
        assert!(!parsed.contains("<span"));
    }

    #[test]
    fn test_title_stays_rich_and_allows_text_elaboration() {
        crate::environment::mock_environment().unwrap();

        let mut metadata = metadata_with_slug("index");
        parse_metadata("title: 中文", &mut metadata).unwrap();

        let parsed = metadata
            .get(KEY_TITLE)
            .and_then(HTMLContent::as_str)
            .unwrap_or_default()
            .to_string();
        assert!(parsed.contains("<span lang=\"zh\">"));
    }

    #[test]
    fn test_taxon_keeps_display_formatting() {
        crate::environment::mock_environment().unwrap();

        let mut metadata = metadata_with_slug("index");
        parse_metadata("taxon: remark", &mut metadata).unwrap();

        let parsed = metadata
            .get(KEY_TAXON)
            .and_then(HTMLContent::as_str)
            .unwrap_or_default()
            .to_string();
        assert_eq!(parsed, "Remark. ");
    }
}
