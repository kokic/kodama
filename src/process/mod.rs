// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use pulldown_cmark::{Event, Tag, TagEnd};

pub mod content;
pub mod embed_markdown;
pub mod figure;
pub mod footnote;
pub mod metadata;
pub mod processer;
pub mod typst_image;

pub fn ignore_paragraph<'e, I>(events: I) -> impl Iterator<Item = Event<'e>>
where
    I: Iterator<Item = Event<'e>>,
{
    events.filter(|e| {
        !matches!(
            e,
            Event::Start(Tag::Paragraph) | Event::End(TagEnd::Paragraph)
        )
    })
}
