// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Spore (@s-cerevisiae)

use pulldown_cmark::Event;

use crate::compiler::section::{EmbedContent, LazyContents, LocalLink};

mod writer;
use writer::HtmlWriter;

#[derive(Debug)]
pub enum EventExtended<'e> {
    CMark(Event<'e>),
    Embed(EmbedContent),
    Local(LocalLink),
}

impl From<LocalLink> for EventExtended<'_> {
    fn from(v: LocalLink) -> Self {
        Self::Local(v)
    }
}

impl From<EmbedContent> for EventExtended<'_> {
    fn from(v: EmbedContent) -> Self {
        Self::Embed(v)
    }
}

impl<'e> From<Event<'e>> for EventExtended<'e> {
    fn from(v: Event<'e>) -> Self {
        Self::CMark(v)
    }
}

pub fn to_contents<'e, I: Iterator<Item = EventExtended<'e>>>(iter: I) -> LazyContents {
    HtmlWriter::new(iter, Vec::new()).run()
}

#[cfg(test)]
mod tests;
