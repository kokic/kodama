// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::collections::HashMap;

use pulldown_cmark::{CowStr, Event, Tag};

use crate::html_flake;

pub struct Footnote<E> {
    events: E,
    entries: HashMap<String, usize>,
}

impl<E> Footnote<E> {
    pub fn process(events: E) -> Self {
        Self {
            events,
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
                let backref = get_back_id(&label);
                let html = format!(
                    r##"<div class="footnote-definition" id="{label}">
  <sup class="footnote-definition-label"><a href="#{backref}">{number}</a></sup>"##,
                );
                Some(Event::Html(html.into()))
            }
            Some(Event::FootnoteReference(label)) => {
                let next_number = self.entries.len() + 1;
                let number = self.entries.entry(label.to_string()).or_insert(next_number);
                let back_id = get_back_id(&label);
                Some(Event::Html(
                    html_flake::footnote_reference(&label, &back_id, *number).into(),
                ))
            }
            e => e,
        }
    }
}

fn get_back_id(s: &CowStr<'_>) -> String {
    format!("{}-back", s)
}
