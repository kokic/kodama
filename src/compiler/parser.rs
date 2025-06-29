// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{collections::HashMap, vec};

use eyre::{eyre, WrapErr};
use pulldown_cmark::{html, CowStr, Event, Options, Tag, TagEnd};

use crate::{
    config::input_path, entry::HTMLMetaData, process::processer::Processer,
    recorder::ParseRecorder, slug::Slug,
};

use super::{
    section::{LazyContent, LazyContents},
    HTMLContent, ShallowSection,
};

pub const OPTIONS: Options = Options::ENABLE_MATH
    .union(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS)
    .union(Options::ENABLE_TABLES)
    .union(Options::ENABLE_SMART_PUNCTUATION)
    .union(Options::ENABLE_FOOTNOTES);

pub fn initialize(
    slug: Slug,
) -> eyre::Result<(String, HashMap<String, HTMLContent>, ParseRecorder)> {
    // global data store
    let mut metadata: HashMap<String, HTMLContent> = HashMap::new();
    let fullname = format!("{}.md", slug);
    metadata.insert("slug".to_string(), HTMLContent::Plain(slug.to_string()));

    // local contents recorder
    let markdown_path = input_path(&fullname);
    let recorder = ParseRecorder::new(fullname);
    std::fs::read_to_string(&markdown_path)
        .map(|markdown_input| (markdown_input, metadata, recorder))
        .wrap_err_with(|| eyre!("Failed to read markdown file `{:?}`", markdown_path))
}

pub fn parse_markdown(slug: Slug) -> eyre::Result<ShallowSection> {
    let mut processers: Vec<Box<dyn Processer>> = vec![
        Box::new(crate::process::footnote::Footnote),
        Box::new(crate::process::figure::Figure),
        Box::new(crate::process::typst_image::TypstImage),
        Box::new(crate::process::katex_compat::KatexCompact),
        Box::new(crate::process::embed_markdown::Embed),
    ];

    let (source, mut metadata, mut recorder) = initialize(slug)?;
    let contents = parse_content(
        &source,
        &mut recorder,
        &mut metadata,
        &mut processers,
        false,
    )?;
    let metadata = HTMLMetaData(metadata);

    return Ok(ShallowSection {
        metadata,
        content: contents,
    });
}

pub fn parse_spanned_markdown(
    markdown_input: &str,
    current_slug: &str,
) -> eyre::Result<HTMLContent> {
    let mut recorder = ParseRecorder::new(current_slug.to_owned());

    let mut processers: Vec<Box<dyn Processer>> = vec![
        Box::new(crate::process::typst_image::TypstImage),
        Box::new(crate::process::katex_compat::KatexCompact),
        Box::new(crate::process::embed_markdown::Embed),
    ];

    parse_content(
        &markdown_input,
        &mut recorder,
        &mut HashMap::new(),
        &mut processers,
        true,
    )
}

pub fn parse_content(
    markdown_input: &str,
    recorder: &mut ParseRecorder,
    metadata: &mut HashMap<String, HTMLContent>,
    processers: &mut Vec<Box<dyn Processer>>,
    ignore_paragraph: bool,
) -> eyre::Result<HTMLContent> {
    let mut contents: LazyContents = vec![];
    let parser = pulldown_cmark::Parser::new_ext(&markdown_input, OPTIONS);

    let mut in_accumulated = false;
    let mut accumulated_events: Vec<Event<'_>> = vec![];

    for mut event in parser {
        match &event {
            Event::Start(tag) => {
                match tag {
                    Tag::Paragraph if ignore_paragraph => continue,
                    Tag::Table(_) => in_accumulated = true,
                    _ => (),
                }

                processers
                    .iter_mut()
                    .for_each(|handler| handler.start(&tag, recorder));
            }

            Event::End(tag) => {
                match tag {
                    TagEnd::Paragraph if ignore_paragraph => continue,
                    TagEnd::Table => {
                        in_accumulated = false;
                        accumulated_events.push(event.clone());
                    }
                    _ => (),
                }

                let mut content: Option<LazyContent> = None;
                for handler in processers.iter_mut() {
                    content = content.or(handler.end(&tag, recorder));
                }

                match content {
                    Some(lazy) => match &lazy {
                        LazyContent::Plain(s) => {
                            event = Event::Html(CowStr::Boxed(s.to_string().into()))
                        }
                        _ => {
                            contents.push(lazy);
                            continue;
                        }
                    },
                    None => (),
                }
            }

            Event::Text(s) => {
                for handler in processers.iter_mut() {
                    handler.text(s, recorder, metadata)?;
                }
            }

            Event::InlineMath(s) => {
                let mut html = String::new();
                processers.iter_mut().for_each(|handler| {
                    handler.inline_math(&s, recorder).map(|s| html = s);
                });
                event = Event::Html(CowStr::Boxed(html.into()));
            }

            Event::DisplayMath(s) => {
                let mut html = String::new();
                processers.iter_mut().for_each(|handler| {
                    handler.display_math(&s, recorder).map(|s| html = s);
                });
                event = Event::Html(CowStr::Boxed(html.into()));
            }

            Event::InlineHtml(s) => {
                processers
                    .iter_mut()
                    .for_each(|handler| handler.inline_html(s, recorder));
            }

            Event::Code(s) => {
                processers
                    .iter_mut()
                    .for_each(|handler| handler.code(s, recorder));
            }

            Event::FootnoteReference(s) => {
                let mut html = String::new();
                processers.iter_mut().for_each(|handler| {
                    handler.footnote(&s, recorder).map(|s| html = s);
                });
                event = Event::Html(CowStr::Boxed(html.into()));
            }
            _ => (),
        };

        if in_accumulated {
            accumulated_events.push(event.clone());
            continue;
        }

        match recorder.is_html_writable() {
            true => {
                let mut html_output = String::new();
                if recorder.data.len() > 0 {
                    html_output = recorder.data.remove(0);
                } else {
                    if accumulated_events.len() > 0 && !in_accumulated {
                        html::push_html(&mut html_output, accumulated_events.clone().into_iter());
                        accumulated_events.clear();
                    } else {
                        html::push_html(&mut html_output, [event].into_iter());
                    }
                }

                // merge plain contents
                match contents.last() {
                    Some(LazyContent::Plain(s)) => {
                        let last_index = contents.len() - 1;
                        contents[last_index] = LazyContent::Plain(s.to_string() + &html_output);
                    }
                    _ => contents.push(LazyContent::Plain(html_output)),
                }
            }
            _ => (),
        }
    }

    if contents.len() == 1 {
        if let LazyContent::Plain(html) = &contents[0] {
            return Ok(HTMLContent::Plain(html.to_string()));
        }
    }
    Ok(HTMLContent::Lazy(contents))
}

mod test {

    #[test]
    fn test_table_td() {
        use std::collections::HashMap;
        use crate::{
            compiler::section::HTMLContent, process::processer::Processer, recorder::ParseRecorder,
        };

        let mut processers: Vec<Box<dyn Processer>> = vec![
            Box::new(crate::process::footnote::Footnote),
            Box::new(crate::process::figure::Figure),
            Box::new(crate::process::typst_image::TypstImage),
            Box::new(crate::process::katex_compat::KatexCompact),
            Box::new(crate::process::embed_markdown::Embed),
        ];

        let source = "| a | b |\n| - | - |\n| c | d |";
        let mut metadata: HashMap<String, HTMLContent> = HashMap::new();
        let mut recorder = ParseRecorder::new("test".to_owned());

        let contents = super::parse_content(
            &source,
            &mut recorder,
            &mut metadata,
            &mut processers,
            false,
        );
        
        assert_eq!(contents.unwrap().as_str().unwrap(), "<table><thead><tr><th>a</th><th>b</th></tr></thead><tbody>\n<tr><td>c</td><td>d</td></tr>\n</tbody></table>\n");
    }
}
