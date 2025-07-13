// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{collections::HashMap, mem};

use eyre::{eyre, WrapErr};
use itertools::Itertools;
use pulldown_cmark::Options;

use crate::{
    config::input_path,
    entry::HTMLMetaData,
    process::{
        content::to_contents, embed_markdown::Embed2, figure::Figure2, footnote::Footnote2,
        ignore_paragraph, katex_compat::KatexCompat2, typst_image::TypstImage2,
    },
    slug::Slug,
};

use super::{section::LazyContent, HTMLContent, ShallowSection};

pub const OPTIONS: Options = Options::ENABLE_MATH
    .union(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS)
    .union(Options::ENABLE_TABLES)
    .union(Options::ENABLE_SMART_PUNCTUATION)
    .union(Options::ENABLE_FOOTNOTES);

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
    fn test_table_td() {
        use crate::compiler::section::HTMLContent;
        use std::collections::HashMap;

        let source = "| a | b |\n| - | - |\n| c | d |";
        let mut metadata: HashMap<String, HTMLContent> = HashMap::new();

        let events = pulldown_cmark::Parser::new_ext(source, OPTIONS);

        let iter = Embed2::new(
            KatexCompat2::new(TypstImage2::new(
                Figure2::new(Footnote2::new(events)),
                Slug::new("-"),
            )),
            &mut metadata,
        );

        let content = iter
            .process_results(|i| HTMLContent::Lazy(to_contents(i)))
            .map(normalize_html_content);

        assert_eq!(content.unwrap().as_str().unwrap(), "<table><thead><tr><th>a</th><th>b</th></tr></thead><tbody>\n<tr><td>c</td><td>d</td></tr>\n</tbody></table>\n");
    }
}
