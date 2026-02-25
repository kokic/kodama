// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::collections::VecDeque;

use pulldown_cmark::{Event, Tag, TagEnd};

pub struct TextElaborator<'e, E> {
    events: E,
    pending: VecDeque<Event<'e>>,
    in_code_block: usize,
}

impl<'e, E> TextElaborator<'e, E> {
    pub fn process(events: E) -> Self {
        Self {
            events,
            pending: VecDeque::new(),
            in_code_block: 0,
        }
    }
}

impl<'e, E: Iterator<Item = Event<'e>>> Iterator for TextElaborator<'e, E> {
    type Item = Event<'e>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pending) = self.pending.pop_front() {
            return Some(pending);
        }

        for event in self.events.by_ref() {
            match event {
                Event::Start(Tag::CodeBlock(_)) => {
                    self.in_code_block += 1;
                    return Some(event);
                }
                Event::End(TagEnd::CodeBlock) => {
                    self.in_code_block = self.in_code_block.saturating_sub(1);
                    return Some(event);
                }
                Event::Text(text) if self.in_code_block == 0 => {
                    self.enqueue_text(text.as_ref());
                    if let Some(next) = self.pending.pop_front() {
                        return Some(next);
                    }
                    continue;
                }
                _ => return Some(event),
            }
        }

        None
    }
}

impl<'e, E> TextElaborator<'e, E> {
    fn enqueue_text(&mut self, text: &str) {
        if !contains_cjk(text) {
            self.pending.push_back(Event::Text(text.to_string().into()));
            return;
        }

        let mut run_start = 0usize;
        let mut run_is_cjk = None;

        for (idx, ch) in text.char_indices() {
            let is_cjk = is_cjk_char(ch);
            match run_is_cjk {
                Some(current) if current != is_cjk => {
                    self.push_run(&text[run_start..idx], current);
                    run_start = idx;
                    run_is_cjk = Some(is_cjk);
                }
                None => {
                    run_is_cjk = Some(is_cjk);
                }
                _ => {}
            }
        }

        if let Some(is_cjk) = run_is_cjk {
            self.push_run(&text[run_start..], is_cjk);
        }
    }

    fn push_run(&mut self, run: &str, is_cjk: bool) {
        if run.is_empty() {
            return;
        }
        if is_cjk {
            self.pending.push_back(Event::InlineHtml(
                r#"<span class="cjk-text">"#.to_string().into(),
            ));
            self.pending.push_back(Event::Text(run.to_string().into()));
            self.pending
                .push_back(Event::InlineHtml("</span>".to_string().into()));
        } else {
            self.pending.push_back(Event::Text(run.to_string().into()));
        }
    }
}

fn contains_cjk(text: &str) -> bool {
    text.chars().any(is_cjk_char)
}

fn is_cjk_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{3000}'..='\u{303F}'
            | '\u{3040}'..='\u{309F}'
            | '\u{30A0}'..='\u{30FF}'
            | '\u{31F0}'..='\u{31FF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{4E00}'..='\u{9FFF}'
            | '\u{A960}'..='\u{A97F}'
            | '\u{AC00}'..='\u{D7AF}'
            | '\u{D7B0}'..='\u{D7FF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{FF00}'..='\u{FFEF}'
    )
}

#[cfg(test)]
mod tests {
    use pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};

    use super::*;

    fn assert_text(event: &Event<'_>, expected: &str) {
        match event {
            Event::Text(text) => assert_eq!(text.as_ref(), expected),
            _ => panic!("expected text event"),
        }
    }

    fn assert_inline_html(event: &Event<'_>, expected: &str) {
        match event {
            Event::InlineHtml(html) => assert_eq!(html.as_ref(), expected),
            _ => panic!("expected inline html event"),
        }
    }

    #[test]
    fn test_wraps_cjk_text_runs() {
        let events = vec![Event::Text("hello 中文 world".into())];
        let actual = TextElaborator::process(events.into_iter()).collect::<Vec<_>>();
        assert_eq!(actual.len(), 5);
        assert_text(&actual[0], "hello ");
        assert_inline_html(&actual[1], r#"<span class="cjk-text">"#);
        assert_text(&actual[2], "中文");
        assert_inline_html(&actual[3], "</span>");
        assert_text(&actual[4], " world");
    }

    #[test]
    fn test_skips_wrapping_inside_code_block() {
        let events = vec![
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced("".into()))),
            Event::Text("中文".into()),
            Event::End(TagEnd::CodeBlock),
        ];
        let actual = TextElaborator::process(events.into_iter()).collect::<Vec<_>>();
        assert_eq!(actual.len(), 3);
        assert!(matches!(
            actual[0],
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(_)))
        ));
        assert_text(&actual[1], "中文");
        assert!(matches!(actual[2], Event::End(TagEnd::CodeBlock)));
    }
}
