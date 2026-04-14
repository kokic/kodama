use pulldown_cmark::{Event, LinkType, Tag, TagEnd};

use super::{to_contents, EventExtended};
use crate::compiler::section::{EmbedContent, LazyContent, LocalLink, SectionOption};

fn joined_plain(contents: &[LazyContent]) -> String {
    let mut out = String::new();
    for content in contents {
        if let LazyContent::Plain(s) = content {
            out.push_str(s);
        }
    }
    out
}

#[test]
fn test_embed_only_paragraph_emits_no_empty_wrapper() {
    let events = vec![
        EventExtended::from(Event::Start(Tag::Paragraph)),
        EventExtended::Embed(EmbedContent {
            url: "/child".to_string(),
            title: None,
            option: SectionOption::default(),
        }),
        EventExtended::from(Event::End(TagEnd::Paragraph)),
    ];

    let contents = to_contents(events.into_iter());
    assert_eq!(contents.len(), 1);
    assert!(matches!(contents.first(), Some(LazyContent::Embed(_))));
}

#[test]
fn test_empty_paragraph_is_dropped() {
    let events = vec![
        EventExtended::from(Event::Start(Tag::Paragraph)),
        EventExtended::from(Event::End(TagEnd::Paragraph)),
    ];
    let contents = to_contents(events.into_iter());
    assert!(contents.is_empty());
}

#[test]
fn test_local_link_stays_wrapped_in_paragraph() {
    let events = vec![
        EventExtended::from(Event::Start(Tag::Paragraph)),
        EventExtended::Local(LocalLink {
            url: "/child".to_string(),
            text: Some("child".to_string()),
        }),
        EventExtended::from(Event::End(TagEnd::Paragraph)),
    ];
    let contents = to_contents(events.into_iter());
    assert_eq!(contents.len(), 3);
    assert!(matches!(
        contents.first(),
        Some(LazyContent::Plain(s)) if s == "<p>"
    ));
    assert!(matches!(contents.get(1), Some(LazyContent::Local(_))));
    assert!(matches!(
        contents.get(2),
        Some(LazyContent::Plain(s)) if s == "</p>\n"
    ));
}

#[test]
fn test_embed_splits_paragraph_without_empty_segments() {
    let events = vec![
        EventExtended::from(Event::Start(Tag::Paragraph)),
        EventExtended::from(Event::Text("before ".into())),
        EventExtended::Embed(EmbedContent {
            url: "/child".to_string(),
            title: None,
            option: SectionOption::default(),
        }),
        EventExtended::from(Event::Text("after".into())),
        EventExtended::from(Event::End(TagEnd::Paragraph)),
    ];

    let contents = to_contents(events.into_iter());
    assert_eq!(contents.len(), 3);
    assert!(matches!(
        contents.first(),
        Some(LazyContent::Plain(s)) if s == "<p>before </p>\n"
    ));
    assert!(matches!(contents.get(1), Some(LazyContent::Embed(_))));
    assert!(matches!(
        contents.get(2),
        Some(LazyContent::Plain(s)) if s == "<p>after</p>\n"
    ));
}

#[test]
fn test_writer_unsafe_link_renders_non_clickable_span() {
    let events = vec![
        EventExtended::from(Event::Start(Tag::Link {
            link_type: LinkType::Inline,
            dest_url: "javascript:alert(1)".into(),
            title: "".into(),
            id: "".into(),
        })),
        EventExtended::from(Event::Text("click".into())),
        EventExtended::from(Event::End(TagEnd::Link)),
    ];

    let contents = to_contents(events.into_iter());
    assert_eq!(
        joined_plain(&contents),
        r#"<span class="link unsafe">click</span>"#
    );
}

#[test]
fn test_writer_unsafe_image_renders_alt_text_without_img_tag() {
    let events = vec![
        EventExtended::from(Event::Start(Tag::Image {
            link_type: LinkType::Inline,
            dest_url: "data:text/html,<svg onload=alert(1)>".into(),
            title: "".into(),
            id: "".into(),
        })),
        EventExtended::from(Event::Text("alt".into())),
        EventExtended::from(Event::End(TagEnd::Image)),
    ];

    let contents = to_contents(events.into_iter());
    let html = joined_plain(&contents);
    assert_eq!(html, "alt");
    assert!(!html.contains("<img"));
}
