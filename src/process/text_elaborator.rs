// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::collections::VecDeque;

use pulldown_cmark::{Event, Tag, TagEnd};

const DEFAULT_HAN_LANG: &str = "zh";

#[derive(Clone, Copy, PartialEq, Eq)]
enum LangTag {
    Zh,
    Ja,
    Ko,
}

impl LangTag {
    fn as_bcp47(self) -> &'static str {
        match self {
            Self::Zh => DEFAULT_HAN_LANG,
            Self::Ja => "ja",
            Self::Ko => "ko",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CharClass {
    Han,
    Japanese,
    Korean,
    Common,
    Other,
}

pub struct TextElaborator<'e, E> {
    events: E,
    pending: VecDeque<Event<'e>>,
    in_code_block: usize,
}

impl<E> TextElaborator<'_, E> {
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

        while let Some(event) = self.events.next() {
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

impl<E> TextElaborator<'_, E> {
    fn enqueue_text(&mut self, text: &str) {
        if !contains_cjk_related(text) {
            self.pending.push_back(Event::Text(text.to_string().into()));
            return;
        }

        let mut run = String::new();
        let mut lang_tag: Option<LangTag> = None;

        for ch in text.chars() {
            match classify_char(ch) {
                CharClass::Other => {
                    if lang_tag.is_some() {
                        self.flush_run(&mut run, lang_tag);
                        lang_tag = None;
                    }
                    run.push(ch);
                }
                CharClass::Common => {
                    run.push(ch);
                }
                CharClass::Han => {
                    if lang_tag.is_none() && !run.is_empty() {
                        self.flush_run(&mut run, None);
                    }
                    if lang_tag.is_none() {
                        lang_tag = Some(LangTag::Zh);
                    }
                    run.push(ch);
                }
                CharClass::Japanese => {
                    if lang_tag == Some(LangTag::Ko) {
                        self.flush_run(&mut run, lang_tag);
                        lang_tag = None;
                    }
                    if lang_tag.is_none() && !run.is_empty() {
                        self.flush_run(&mut run, None);
                    }
                    if matches!(lang_tag, None | Some(LangTag::Zh)) {
                        lang_tag = Some(LangTag::Ja);
                    }
                    run.push(ch);
                }
                CharClass::Korean => {
                    if lang_tag == Some(LangTag::Ja) {
                        self.flush_run(&mut run, lang_tag);
                        lang_tag = None;
                    }
                    if lang_tag.is_none() && !run.is_empty() {
                        self.flush_run(&mut run, None);
                    }
                    if matches!(lang_tag, None | Some(LangTag::Zh)) {
                        lang_tag = Some(LangTag::Ko);
                    }
                    run.push(ch);
                }
            }
        }

        self.flush_run(&mut run, lang_tag);
    }

    fn flush_run(&mut self, run: &mut String, lang_tag: Option<LangTag>) {
        if run.is_empty() {
            return;
        }
        let text = std::mem::take(run);
        match lang_tag {
            Some(lang_tag) => {
                self.pending.push_back(Event::InlineHtml(
                    format!(r#"<span lang="{}">"#, lang_tag.as_bcp47()).into(),
                ));
                self.pending.push_back(Event::Text(text.into()));
                self.pending
                    .push_back(Event::InlineHtml("</span>".to_string().into()));
            }
            None => {
                self.pending.push_back(Event::Text(text.into()));
            }
        }
    }
}

fn contains_cjk_related(text: &str) -> bool {
    text.chars().any(|ch| classify_char(ch) != CharClass::Other)
}

fn classify_char(ch: char) -> CharClass {
    if is_japanese_char(ch) {
        CharClass::Japanese
    } else if is_korean_char(ch) {
        CharClass::Korean
    } else if is_han_char(ch) {
        CharClass::Han
    } else if is_cjk_common_char(ch) {
        CharClass::Common
    } else {
        CharClass::Other
    }
}

fn is_han_char(ch: char) -> bool {
    matches!(ch, '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}')
}

fn is_japanese_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{3040}'..='\u{309F}'
            | '\u{30A0}'..='\u{30FF}'
            | '\u{31F0}'..='\u{31FF}'
            | '\u{FF66}'..='\u{FF9D}'
            | '\u{1B000}'..='\u{1B0FF}'
            | '\u{1B100}'..='\u{1B12F}'
    )
}

fn is_korean_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{1100}'..='\u{11FF}'
            | '\u{3130}'..='\u{318F}'
            | '\u{A960}'..='\u{A97F}'
            | '\u{AC00}'..='\u{D7AF}'
            | '\u{D7B0}'..='\u{D7FF}'
            | '\u{FFA0}'..='\u{FFDC}'
    )
}

fn is_cjk_common_char(ch: char) -> bool {
    matches!(
        ch,
        '\u{3000}'..='\u{303F}' | '\u{FE10}'..='\u{FE6F}' | '\u{FF00}'..='\u{FF65}'
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
        assert_inline_html(&actual[1], r#"<span lang="zh">"#);
        assert_text(&actual[2], "中文");
        assert_inline_html(&actual[3], "</span>");
        assert_text(&actual[4], " world");
    }

    #[test]
    fn test_uses_japanese_lang_when_kana_present() {
        let events = vec![Event::Text("答えの潜む琥珀の太阳".into())];
        let actual = TextElaborator::process(events.into_iter()).collect::<Vec<_>>();
        assert_eq!(actual.len(), 3);
        assert_inline_html(&actual[0], r#"<span lang="ja">"#);
        assert_text(&actual[1], "答えの潜む琥珀の太阳");
        assert_inline_html(&actual[2], "</span>");
    }

    #[test]
    fn test_uses_korean_lang_when_hangul_present() {
        let events = vec![Event::Text("한글漢字".into())];
        let actual = TextElaborator::process(events.into_iter()).collect::<Vec<_>>();
        assert_eq!(actual.len(), 3);
        assert_inline_html(&actual[0], r#"<span lang="ko">"#);
        assert_text(&actual[1], "한글漢字");
        assert_inline_html(&actual[2], "</span>");
    }

    #[test]
    fn test_splits_between_japanese_and_korean_runs() {
        let events = vec![Event::Text("かな한글".into())];
        let actual = TextElaborator::process(events.into_iter()).collect::<Vec<_>>();
        assert_eq!(actual.len(), 6);
        assert_inline_html(&actual[0], r#"<span lang="ja">"#);
        assert_text(&actual[1], "かな");
        assert_inline_html(&actual[2], "</span>");
        assert_inline_html(&actual[3], r#"<span lang="ko">"#);
        assert_text(&actual[4], "한글");
        assert_inline_html(&actual[5], "</span>");
    }

    #[test]
    fn test_uses_default_zh_lang_for_han_only_text() {
        let events = vec![Event::Text("中文".into())];
        let actual = TextElaborator::process(events.into_iter()).collect::<Vec<_>>();
        assert_eq!(actual.len(), 3);
        assert_inline_html(&actual[0], r#"<span lang="zh">"#);
        assert_text(&actual[1], "中文");
        assert_inline_html(&actual[2], "</span>");
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
