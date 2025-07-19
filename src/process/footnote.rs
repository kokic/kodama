// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::collections::HashMap;

use pulldown_cmark::{Event, Tag};

use crate::{html_flake, slug::Slug};

pub struct Footnote<E> {
    events: E,
    current_slug: Slug,
    entries: HashMap<String, usize>,
}

impl<E> Footnote<E> {
    pub fn process(events: E, current_slug: Slug) -> Self {
        Self {
            events,
            current_slug,
            entries: HashMap::new(),
        }
    }
}

impl<'e, E: Iterator<Item = Event<'e>>> Iterator for Footnote<E> {
    type Item = Event<'e>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.events.next() {
            Some(Event::Start(Tag::FootnoteDefinition(label))) => {
                let next_number = self.entries.len() + 1;
                let number = self.entries.entry(label.to_string()).or_insert(next_number);
                let footnote_id = get_footnote_id(self.current_slug, &label);
                let back_id = get_back_id(self.current_slug, &label);
                let html = format!(
                    r##"<div class="footnote-definition" id="{footnote_id}">
  <sup class="footnote-definition-label"><a href="#{back_id}">{number}</a></sup>"##,
                );
                Some(Event::Html(html.into()))
            }
            Some(Event::FootnoteReference(label)) => {
                let next_number = self.entries.len() + 1;
                let number = self.entries.entry(label.to_string()).or_insert(next_number);
                let footnote_id = get_footnote_id(self.current_slug, &label);
                let back_id = get_back_id(self.current_slug, &label);
                Some(Event::Html(
                    html_flake::footnote_reference(&footnote_id, &back_id, *number).into(),
                ))
            }
            e => e,
        }
    }
}

fn slug_to_attr(slug: Slug) -> String {
    slug.as_str().replace("/", "_")
}

fn get_footnote_id(slug: Slug, label: &str) -> String {
    format!("{}_{}", slug_to_attr(slug), label)
}

fn get_back_id(slug: Slug, label: &str) -> String {
    format!("{}-back", get_footnote_id(slug, label))
}
