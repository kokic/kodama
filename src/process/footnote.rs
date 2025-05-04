// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::collections::HashMap;

use pulldown_cmark::{CowStr, Event, Tag};

use crate::{html_flake, recorder::ParseRecorder};

use super::processer::Processer;

pub struct Footnote;

pub struct Footnote2<E> {
    events: E,
    entries: HashMap<String, usize>,
}

impl<E> Footnote2<E> {
    pub fn new(events: E) -> Self {
        Self { events, entries: HashMap::new() }
    }
}

impl<'e, E: Iterator<Item = Event<'e>>> Iterator for Footnote2<E> {
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

impl Processer for Footnote {
    fn footnote(
        &self,
        s: &CowStr<'_>,
        recorder: &mut crate::recorder::ParseRecorder,
    ) -> Option<String> {
        let name = s.to_string();
        let len = recorder.footnote_counter.len() + 1;
        let number = recorder.footnote_counter.entry(name.into()).or_insert(len);
        let back_id = get_back_id(s);
        Some(html_flake::footnote_reference(s, &back_id, *number))
    }

    fn start(&mut self, tag: &Tag<'_>, recorder: &mut ParseRecorder) {
        match tag {
            Tag::FootnoteDefinition(s) => {
                let name = s.to_string();
                let len = recorder.footnote_counter.len() + 1;
                let number = recorder.footnote_counter.entry(name.into()).or_insert(len);

                let back_href = format!("#{}", get_back_id(s));
                let html = format!(
                    r#"<div class="footnote-definition" id="{}">
  <sup class="footnote-definition-label"><a href="{}">{}</a></sup>"#,
                    s, back_href, number
                );
                recorder.push(html);
            }
            _ => (),
        }
    }
}

fn get_back_id(s: &CowStr<'_>) -> String {
    format!("{}-back", s)
}
