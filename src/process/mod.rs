// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use pulldown_cmark::{Event, Tag, TagEnd};

pub mod content;
pub mod embed_markdown;
pub mod figure;
pub mod footnote;
pub mod metadata;
pub mod path_resolution;
pub mod processer;
pub mod text_elaborator;
pub mod typst_image;

pub fn filter_raw_html<'e, I>(events: I, allow_unsafe_html: bool) -> impl Iterator<Item = Event<'e>>
where
    I: Iterator<Item = Event<'e>>,
{
    events.filter_map(move |event| {
        if allow_unsafe_html {
            return Some(event);
        }

        match event {
            Event::Html(_) | Event::InlineHtml(_) => None,
            _ => Some(event),
        }
    })
}

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

#[cfg(test)]
mod tests {
    use pulldown_cmark::Event;

    use super::filter_raw_html;

    #[test]
    fn test_filter_raw_html_drops_html_when_unsafe_html_is_disabled() {
        let events = vec![
            Event::Text("safe ".into()),
            Event::InlineHtml("<span>".into()),
            Event::Text("ok".into()),
            Event::InlineHtml("</span>".into()),
        ];

        let actual: Vec<_> = filter_raw_html(events.into_iter(), false).collect();
        assert!(matches!(&actual[0], Event::Text(text) if text.as_ref() == "safe "));
        assert!(matches!(&actual[1], Event::Text(text) if text.as_ref() == "ok"));
        assert_eq!(actual.len(), 2);
    }

    #[test]
    fn test_filter_raw_html_preserves_html_when_unsafe_html_is_enabled() {
        let events = vec![
            Event::Text("safe ".into()),
            Event::InlineHtml("<span>".into()),
            Event::Text("ok".into()),
            Event::InlineHtml("</span>".into()),
        ];

        let actual: Vec<_> = filter_raw_html(events.into_iter(), true).collect();
        assert!(matches!(&actual[0], Event::Text(text) if text.as_ref() == "safe "));
        assert!(
            matches!(&actual[1], Event::InlineHtml(text) if text.as_ref() == "<span>")
        );
        assert!(matches!(&actual[2], Event::Text(text) if text.as_ref() == "ok"));
        assert!(
            matches!(&actual[3], Event::InlineHtml(text) if text.as_ref() == "</span>")
        );
    }
}
