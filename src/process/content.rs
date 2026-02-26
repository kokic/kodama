// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Spore (@s-cerevisiae)

use std::{collections::HashMap, fmt::Write};

use pulldown_cmark::{
    Alignment, BlockQuoteKind, CodeBlockKind, CowStr, Event, LinkType, Tag, TagEnd,
};
use pulldown_cmark_escape::{escape_href, escape_html, escape_html_body_text};

use crate::compiler::section::{EmbedContent, LazyContent, LazyContents, LocalLink};

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

enum TableState {
    Head,
    Body,
}

struct HtmlWriter<'e, I> {
    /// Iterator supplying events.
    iter: I,

    /// Contents being written.
    contents: LazyContents,

    /// Whether or not the last write wrote a newline.
    end_newline: bool,

    /// Whether if inside a metadata block (text should not be written)
    in_non_writing_block: bool,

    table_state: TableState,
    table_alignments: Vec<Alignment>,
    table_cell_index: usize,
    numbers: HashMap<CowStr<'e>, usize>,
    in_paragraph: bool,
    paragraph_started: bool,
}

impl<'e, I> HtmlWriter<'e, I>
where
    I: Iterator<Item = EventExtended<'e>>,
{
    fn new(iter: I, contents: LazyContents) -> Self {
        Self {
            iter,
            contents,
            end_newline: true,
            in_non_writing_block: false,
            table_state: TableState::Head,
            table_alignments: vec![],
            table_cell_index: 0,
            numbers: HashMap::new(),
            in_paragraph: false,
            paragraph_started: false,
        }
    }

    fn append_str(&mut self, str: &str) {
        match self.contents.last_mut() {
            Some(LazyContent::Plain(s)) => s.push_str(str),
            _ => self.contents.push(LazyContent::Plain(str.into())),
        }
    }

    fn writer(&mut self) -> &mut String {
        match self.contents.last() {
            Some(LazyContent::Plain(_)) => (),
            _ => self.contents.push(LazyContent::Plain(String::new())),
        }
        let Some(LazyContent::Plain(s)) = self.contents.last_mut() else {
            unreachable!();
        };
        s
    }

    /// Writes a new line.
    #[inline]
    fn write_newline(&mut self) {
        self.end_newline = true;
        self.append_str("\n");
    }

    /// Writes a buffer, and tracks whether or not a newline was written.
    #[inline]
    fn write(&mut self, s: &str) {
        self.append_str(s);

        if !s.is_empty() {
            self.end_newline = s.ends_with('\n');
        }
    }

    #[inline]
    fn ensure_paragraph_started(&mut self) {
        if !self.in_paragraph || self.paragraph_started {
            return;
        }
        if self.end_newline {
            self.write("<p>");
        } else {
            self.write("\n<p>");
        }
        self.paragraph_started = true;
    }

    #[inline]
    fn close_paragraph_if_started(&mut self) {
        if !self.in_paragraph || !self.paragraph_started {
            return;
        }
        self.write("</p>\n");
        self.paragraph_started = false;
    }

    fn run(mut self) -> LazyContents {
        use Event::*;
        while let Some(event_ext) = self.iter.next() {
            let event = match event_ext {
                EventExtended::CMark(event) => event,
                EventExtended::Embed(embed_content) => {
                    // Embed expands to block-level section content in later stages.
                    // Do not leave an empty paragraph wrapper around it.
                    self.close_paragraph_if_started();
                    self.contents.push(LazyContent::Embed(embed_content));
                    continue;
                }
                EventExtended::Local(local_link) => {
                    self.ensure_paragraph_started();
                    self.contents.push(LazyContent::Local(local_link));
                    continue;
                }
            };
            match event {
                Start(tag) => {
                    if !matches!(tag, Tag::Paragraph) {
                        self.ensure_paragraph_started();
                    }
                    self.start_tag(tag);
                }
                End(tag) => {
                    self.end_tag(tag);
                }
                Text(text) => {
                    if !self.in_non_writing_block {
                        self.ensure_paragraph_started();
                        escape_html_body_text(self.writer(), &text).unwrap();
                        self.end_newline = text.ends_with('\n');
                    }
                }
                Code(text) => {
                    self.ensure_paragraph_started();
                    self.write("<code>");
                    escape_html_body_text(self.writer(), &text).unwrap();
                    self.write("</code>");
                }
                InlineMath(text) => {
                    self.ensure_paragraph_started();
                    self.write(r#"<span class="math math-inline">"#);
                    escape_html(self.writer(), &text).unwrap();
                    self.write("</span>");
                }
                DisplayMath(text) => {
                    self.ensure_paragraph_started();
                    self.write(r#"<span class="math math-display">"#);
                    escape_html(self.writer(), &text).unwrap();
                    self.write("</span>");
                }
                Html(html) | InlineHtml(html) => {
                    self.ensure_paragraph_started();
                    self.write(&html);
                }
                SoftBreak => {
                    if !self.paragraph_started {
                        continue;
                    }
                    self.write_newline();
                }
                HardBreak => {
                    self.ensure_paragraph_started();
                    self.write("<br />\n");
                }
                Rule => {
                    self.ensure_paragraph_started();
                    if self.end_newline {
                        self.write("<hr />\n");
                    } else {
                        self.write("\n<hr />\n");
                    }
                }
                FootnoteReference(name) => {
                    self.ensure_paragraph_started();
                    let len = self.numbers.len() + 1;
                    self.write("<sup class=\"footnote-reference\"><a href=\"#");
                    escape_html(self.writer(), &name).unwrap();
                    self.write("\">");
                    let number = *self.numbers.entry(name).or_insert(len);
                    write!(self.writer(), "{}", number).unwrap();
                    self.write("</a></sup>");
                }
                TaskListMarker(true) => {
                    self.ensure_paragraph_started();
                    self.write("<input disabled=\"\" type=\"checkbox\" checked=\"\"/>\n");
                }
                TaskListMarker(false) => {
                    self.ensure_paragraph_started();
                    self.write("<input disabled=\"\" type=\"checkbox\"/>\n");
                }
            }
        }
        self.contents
    }

    /// Writes the start of an HTML tag.
    fn start_tag(&mut self, tag: Tag<'e>) {
        match tag {
            Tag::HtmlBlock => (),
            Tag::Paragraph => {
                self.in_paragraph = true;
                self.paragraph_started = false;
            }
            Tag::Heading {
                level,
                id,
                classes,
                attrs,
            } => {
                if self.end_newline {
                    self.write("<");
                } else {
                    self.write("\n<");
                }
                write!(self.writer(), "{}", level).unwrap();
                if let Some(id) = id {
                    self.write(" id=\"");
                    escape_html(self.writer(), &id).unwrap();
                    self.write("\"");
                }
                let mut classes = classes.iter();
                if let Some(class) = classes.next() {
                    self.write(" class=\"");
                    escape_html(self.writer(), class).unwrap();
                    for class in classes {
                        self.write(" ");
                        escape_html(self.writer(), class).unwrap();
                    }
                    self.write("\"");
                }
                for (attr, value) in attrs {
                    self.write(" ");
                    escape_html(self.writer(), &attr).unwrap();
                    if let Some(val) = value {
                        self.write("=\"");
                        escape_html(self.writer(), &val).unwrap();
                        self.write("\"");
                    } else {
                        self.write("=\"\"");
                    }
                }
                self.write(">")
            }
            Tag::Table(alignments) => {
                self.table_alignments = alignments;
                self.write("<table>")
            }
            Tag::TableHead => {
                self.table_state = TableState::Head;
                self.table_cell_index = 0;
                self.write("<thead><tr>")
            }
            Tag::TableRow => {
                self.table_cell_index = 0;
                self.write("<tr>")
            }
            Tag::TableCell => {
                match self.table_state {
                    TableState::Head => {
                        self.write("<th");
                    }
                    TableState::Body => {
                        self.write("<td");
                    }
                }
                match self.table_alignments.get(self.table_cell_index) {
                    Some(&Alignment::Left) => self.write(" style=\"text-align: left\">"),
                    Some(&Alignment::Center) => self.write(" style=\"text-align: center\">"),
                    Some(&Alignment::Right) => self.write(" style=\"text-align: right\">"),
                    _ => self.write(">"),
                }
            }
            Tag::BlockQuote(kind) => {
                let class_str = match kind {
                    None => "",
                    Some(kind) => match kind {
                        BlockQuoteKind::Note => " class=\"markdown-alert-note\"",
                        BlockQuoteKind::Tip => " class=\"markdown-alert-tip\"",
                        BlockQuoteKind::Important => " class=\"markdown-alert-important\"",
                        BlockQuoteKind::Warning => " class=\"markdown-alert-warning\"",
                        BlockQuoteKind::Caution => " class=\"markdown-alert-caution\"",
                    },
                };
                if self.end_newline {
                    self.write(&format!("<blockquote{}>\n", class_str))
                } else {
                    self.write(&format!("\n<blockquote{}>\n", class_str))
                }
            }
            Tag::CodeBlock(info) => {
                if !self.end_newline {
                    self.write_newline();
                }
                match info {
                    CodeBlockKind::Fenced(info) => {
                        let lang = info.split(' ').next().unwrap();
                        if lang.is_empty() {
                            self.write("<pre><code>")
                        } else {
                            self.write("<pre><code class=\"language-");
                            escape_html(self.writer(), lang).unwrap();
                            self.write("\">")
                        }
                    }
                    CodeBlockKind::Indented => self.write("<pre><code>"),
                }
            }
            Tag::List(Some(1)) => {
                if self.end_newline {
                    self.write("<ol>\n")
                } else {
                    self.write("\n<ol>\n")
                }
            }
            Tag::List(Some(start)) => {
                if self.end_newline {
                    self.write("<ol start=\"");
                } else {
                    self.write("\n<ol start=\"");
                }
                write!(self.writer(), "{}", start).unwrap();
                self.write("\">\n")
            }
            Tag::List(None) => {
                if self.end_newline {
                    self.write("<ul>\n")
                } else {
                    self.write("\n<ul>\n")
                }
            }
            Tag::Item => {
                if self.end_newline {
                    self.write("<li>")
                } else {
                    self.write("\n<li>")
                }
            }
            Tag::DefinitionList => {
                if self.end_newline {
                    self.write("<dl>\n")
                } else {
                    self.write("\n<dl>\n")
                }
            }
            Tag::DefinitionListTitle => {
                if self.end_newline {
                    self.write("<dt>")
                } else {
                    self.write("\n<dt>")
                }
            }
            Tag::DefinitionListDefinition => {
                if self.end_newline {
                    self.write("<dd>")
                } else {
                    self.write("\n<dd>")
                }
            }
            Tag::Emphasis => self.write("<em>"),
            Tag::Strong => self.write("<strong>"),
            Tag::Strikethrough => self.write("<del>"),
            Tag::Link {
                link_type: LinkType::Email,
                dest_url,
                title,
                id: _,
            } => {
                self.write("<a href=\"mailto:");
                escape_href(self.writer(), &dest_url).unwrap();
                if !title.is_empty() {
                    self.write("\" title=\"");
                    escape_html(self.writer(), &title).unwrap();
                }
                self.write("\">")
            }
            Tag::Link {
                link_type: _,
                dest_url,
                title,
                id: _,
            } => {
                self.write("<a href=\"");
                escape_href(self.writer(), &dest_url).unwrap();
                if !title.is_empty() {
                    self.write("\" title=\"");
                    escape_html(self.writer(), &title).unwrap();
                }
                self.write("\">")
            }
            Tag::Image {
                link_type: _,
                dest_url,
                title,
                id: _,
            } => {
                self.write("<img src=\"");
                escape_href(self.writer(), &dest_url).unwrap();
                self.write("\" alt=\"");
                self.raw_text();
                if !title.is_empty() {
                    self.write("\" title=\"");
                    escape_html(self.writer(), &title).unwrap();
                }
                self.write("\" />")
            }
            Tag::FootnoteDefinition(name) => {
                if self.end_newline {
                    self.write("<div class=\"footnote-definition\" id=\"");
                } else {
                    self.write("\n<div class=\"footnote-definition\" id=\"");
                }
                escape_html(self.writer(), &name).unwrap();
                self.write("\"><sup class=\"footnote-definition-label\">");
                let len = self.numbers.len() + 1;
                let number = *self.numbers.entry(name).or_insert(len);
                write!(self.writer(), "{}", number).unwrap();
                self.write("</sup>")
            }
            Tag::MetadataBlock(_) => {
                self.in_non_writing_block = true;
            }
        }
    }

    fn end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::HtmlBlock => {}
            TagEnd::Paragraph => {
                self.close_paragraph_if_started();
                self.in_paragraph = false;
                self.paragraph_started = false;
            }
            TagEnd::Heading(level) => {
                self.write("</");
                write!(self.writer(), "{}", level).unwrap();
                self.write(">\n");
            }
            TagEnd::Table => {
                self.write("</tbody></table>\n");
            }
            TagEnd::TableHead => {
                self.write("</tr></thead><tbody>\n");
                self.table_state = TableState::Body;
            }
            TagEnd::TableRow => {
                self.write("</tr>\n");
            }
            TagEnd::TableCell => {
                match self.table_state {
                    TableState::Head => {
                        self.write("</th>");
                    }
                    TableState::Body => {
                        self.write("</td>");
                    }
                }
                self.table_cell_index += 1;
            }
            TagEnd::BlockQuote(_) => {
                self.write("</blockquote>\n");
            }
            TagEnd::CodeBlock => {
                self.write("</code></pre>\n");
            }
            TagEnd::List(true) => {
                self.write("</ol>\n");
            }
            TagEnd::List(false) => {
                self.write("</ul>\n");
            }
            TagEnd::Item => {
                self.write("</li>\n");
            }
            TagEnd::DefinitionList => {
                self.write("</dl>\n");
            }
            TagEnd::DefinitionListTitle => {
                self.write("</dt>\n");
            }
            TagEnd::DefinitionListDefinition => {
                self.write("</dd>\n");
            }
            TagEnd::Emphasis => {
                self.write("</em>");
            }
            TagEnd::Strong => {
                self.write("</strong>");
            }
            TagEnd::Strikethrough => {
                self.write("</del>");
            }
            TagEnd::Link => {
                self.write("</a>");
            }
            TagEnd::Image => (), // shouldn't happen, handled in start
            TagEnd::FootnoteDefinition => {
                self.write("</div>\n");
            }
            TagEnd::MetadataBlock(_) => {
                self.in_non_writing_block = false;
            }
        }
    }

    // run raw text, consuming end tag
    fn raw_text(&mut self) {
        use Event::*;
        let mut nest = 0;
        while let Some(event_ext) = self.iter.next() {
            let event = match event_ext {
                EventExtended::CMark(event) => event,
                EventExtended::Embed(embed_content) => {
                    self.contents.push(LazyContent::Embed(embed_content));
                    continue;
                }
                EventExtended::Local(local_link) => {
                    self.contents.push(LazyContent::Local(local_link));
                    continue;
                }
            };
            match event {
                Start(_) => nest += 1,
                End(_) => {
                    if nest == 0 {
                        break;
                    }
                    nest -= 1;
                }
                Html(_) => {}
                InlineHtml(text) | Code(text) | Text(text) => {
                    // Don't use escape_html_body_text here.
                    // The output of this function is used in the `alt` attribute.
                    escape_html(self.writer(), &text).unwrap();
                    self.end_newline = text.ends_with('\n');
                }
                InlineMath(text) => {
                    self.write("$");
                    escape_html(self.writer(), &text).unwrap();
                    self.write("$");
                }
                DisplayMath(text) => {
                    self.write("$$");
                    escape_html(self.writer(), &text).unwrap();
                    self.write("$$");
                }
                SoftBreak | HardBreak | Rule => {
                    self.write(" ");
                }
                FootnoteReference(name) => {
                    let len = self.numbers.len() + 1;
                    let number = *self.numbers.entry(name).or_insert(len);
                    write!(self.writer(), "[{}]", number).unwrap();
                }
                TaskListMarker(true) => self.write("[x]"),
                TaskListMarker(false) => self.write("[ ]"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use pulldown_cmark::{Event, Tag, TagEnd};

    use super::{to_contents, EventExtended};
    use crate::compiler::section::{EmbedContent, LazyContent, LocalLink, SectionOption};

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
}
