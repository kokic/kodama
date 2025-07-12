// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use pulldown_cmark::{Event, Tag, TagEnd};

use crate::{
    compiler::section::{HTMLContent, LazyContent},
    recorder::{ParseRecorder, State},
};

use super::processer::Processer;

pub struct Figure;

pub struct Figure2<E> {
    events: E,
    title: String,
    dest_url: Option<String>,
}

impl<E> Figure2<E> {
    pub fn new(events: E) -> Self {
        Self {
            events,
            title: String::new(),
            dest_url: None,
        }
    }
}

impl<'e, E: Iterator<Item = Event<'e>>> Iterator for Figure2<E> {
    type Item = Event<'e>;

    fn next(&mut self) -> Option<Self::Item> {
        for e in self.events.by_ref() {
            match e {
                Event::Start(Tag::Image { dest_url, .. }) => self.dest_url = Some(dest_url.into()),
                Event::Text(text) if self.dest_url.is_some() => self.title.push_str(&text),
                Event::End(TagEnd::Image) => {
                    let title_escaped = htmlize::escape_attribute(&self.title);
                    let html = format!(
                        r#"<img src="{}" title="{}" alt="{}">"#,
                        self.dest_url.take().unwrap_or_default(),
                        title_escaped,
                        title_escaped,
                    );
                    self.title.clear();
                    return Some(Event::Html(html.into()));
                }
                _ => return Some(e),
            }
        }

        None
    }
}

impl Processer for Figure {
    fn start(&mut self, tag: &Tag<'_>, recorder: &mut ParseRecorder) {
        match tag {
            Tag::Image {
                link_type: _,
                dest_url,
                title: _,
                id: _,
            } => {
                recorder.enter(State::Figure);
                recorder.push(dest_url.to_string()); // [0]
            }
            _ => (),
        }
    }

    fn end(&mut self, _tag: &TagEnd, recorder: &mut ParseRecorder) -> Option<LazyContent> {
        if recorder.state == State::Figure {
            let url = recorder.data.get(0).map_or("", |s| s);
            let alt = recorder.data.get(1).map_or("", |s| s);
            let html = format!(r#"<img src="{}" title="{}" alt="{}">"#, url, alt, alt);
            recorder.exit();
            return Some(LazyContent::Plain(html));
        }
        None
    }

    fn text(
        &self,
        s: &pulldown_cmark::CowStr<'_>,
        recorder: &mut ParseRecorder,
        _metadata: &mut std::collections::HashMap<String, HTMLContent>,
    ) -> eyre::Result<()> {
        if recorder.state == State::Figure {
            recorder.push(s.to_string()); // [1]: alt text
        }
        Ok(())
    }
}
