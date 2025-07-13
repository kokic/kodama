// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{collections::HashMap, mem, vec};

use eyre::{eyre, WrapErr};
use itertools::Itertools;
use pulldown_cmark::{html, CowStr, Event, Options, Tag, TagEnd};

use crate::{
    config::input_path,
    entry::HTMLMetaData,
    process::{
        content::to_contents, embed_markdown::Embed2, figure::Figure2, footnote::Footnote2,
        ignore_paragraph, katex_compat::KatexCompat2, processer::Processer,
        typst_image::TypstImage2,
    },
    recorder::ParseRecorder,
    slug::Slug,
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
        .wrap_err_with(|| eyre!("failed to read markdown file `{markdown_path}`"))
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

pub fn initialize2(slug: Slug) -> eyre::Result<(String, HashMap<String, HTMLContent>)> {
    // global data store
    let mut metadata: HashMap<String, HTMLContent> = HashMap::new();
    let fullname = format!("{}.md", slug);
    metadata.insert("slug".to_string(), HTMLContent::Plain(slug.to_string()));

    // local contents recorder
    let markdown_path = input_path(&fullname);
    std::fs::read_to_string(&markdown_path)
        .map(|markdown_input| (markdown_input, metadata))
        .wrap_err_with(|| eyre!("failed to read markdown file `{markdown_path}`"))
}

pub fn parse_markdown2(slug: Slug) -> eyre::Result<ShallowSection> {
    let (source, mut metadata) = initialize2(slug)?;
    let events = pulldown_cmark::Parser::new_ext(&source, OPTIONS);

    let iter = Embed2::new(
        KatexCompat2::new(TypstImage2::new(Figure2::new(Footnote2::new(events)), slug)),
        &mut metadata,
    );

    let content = iter
        .process_results(|i| HTMLContent::Lazy(to_contents(i)))
        .map(normalize_html_content)?;
    let metadata = HTMLMetaData(metadata);

    Ok(ShallowSection { metadata, content })
}

pub fn parse_spanned_markdown2(markdown_input: &str, slug: Slug) -> eyre::Result<HTMLContent> {
    let events = pulldown_cmark::Parser::new_ext(markdown_input, OPTIONS);
    let events = ignore_paragraph(events);
    let mut metadata = HashMap::new();
    let iter = Embed2::new(
        KatexCompat2::new(TypstImage2::new(events, slug)),
        &mut metadata,
    );
    iter.process_results(|i| HTMLContent::Lazy(to_contents(i)))
        .map(normalize_html_content)
}

fn normalize_html_content(mut content: HTMLContent) -> HTMLContent {
    match &mut content {
        HTMLContent::Lazy(lazy_contents) => {
            if let [LazyContent::Plain(html)] = lazy_contents.as_mut_slice() {
                HTMLContent::Plain(mem::take(html))
            } else {
                content
            }
        }
        _ => content,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markdown2() -> eyre::Result<()> {
        let input = r#"有两个正方体, 一个边长为 $1$, 另一个边长为 $2$. 请找到另外两个边长为有理数的正方体使它们的体积总和相同. 换言之, 求下述方程的一组 (正) 有理解: 

$$ x^3 + y^3 \eqq 9 \quad \color{gray}{(= \quad 1^3+2^3)} $$

我们先画出 $x^3 + y^3 = 9$. 然后由已知的 $P=(1,2)$ 出发做切线得到 $2P$, $4P$. 

[+](/mille-plateaux/canterbury.md#:embed)

如图, 随后注意到 $8P$ 恰好位于 $x > 0,y > 0$ 的区域, 现在写出其坐标

$$ 8P = \left(\frac{1243617733990094836481}{609623835676137297449}, \frac{487267171714352336560}{609623835676137297449}\right) $$"#;
        let events = pulldown_cmark::Parser::new_ext(input, OPTIONS);

        let mut metadata = HashMap::new();
        let iter = Embed2::new(
            KatexCompat2::new(TypstImage2::new(
                Figure2::new(Footnote2::new(events)),
                Slug::new("-"),
            )),
            &mut metadata,
        );

        let content = iter.process_results(|i| HTMLContent::Lazy(to_contents(i)))?;
        println!("{content:?}");
        Ok(())
    }
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

        if recorder.is_html_writable() {
            let mut html_output = String::new();
            if !recorder.data.is_empty() {
                html_output = recorder.data.remove(0);
            } else if !accumulated_events.is_empty() && !in_accumulated {
                html::push_html(&mut html_output, accumulated_events.clone().into_iter());
                accumulated_events.clear();
            } else {
                html::push_html(&mut html_output, [event].into_iter());
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
        use crate::{
            compiler::section::HTMLContent, process::processer::Processer, recorder::ParseRecorder,
        };
        use std::collections::HashMap;

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
