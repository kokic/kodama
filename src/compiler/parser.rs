// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::vec;

use eyre::{eyre, WrapErr};
use itertools::Itertools;
use pulldown_cmark::Options;

use crate::{
    config::input_path, entry::HTMLMetaData, ordered_map::OrderedMap,
    process::processer::Processer, recorder::ParseRecorder, slug::Slug,
};

use super::{section::LazyContent, HTMLContent, ShallowSection};

pub const OPTIONS: Options = Options::ENABLE_MATH
    .union(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS)
    .union(Options::ENABLE_TABLES)
    .union(Options::ENABLE_SMART_PUNCTUATION)
    .union(Options::ENABLE_FOOTNOTES);

pub fn initialize(slug: Slug) -> eyre::Result<(String, OrderedMap<String, HTMLContent>)> {
    // global data store
    let mut metadata: OrderedMap<String, HTMLContent> = OrderedMap::new();
    let fullname = format!("{}.md", slug);
    metadata.insert("slug".to_string(), HTMLContent::Plain(slug.to_string()));

    // local contents recorder
    let markdown_path = input_path(&fullname);
    std::fs::read_to_string(&markdown_path)
        .map(|markdown_input| (markdown_input, metadata))
        .wrap_err_with(|| eyre!("failed to read markdown file `{:?}`", markdown_path))
}

pub fn parse_markdown(slug: Slug) -> eyre::Result<ShallowSection> {
    let (source, mut metadata) = initialize(slug)?;
    let events = pulldown_cmark::Parser::new_ext(&source, OPTIONS);

    let content = Metadata::process(events, &mut metadata)
        .process_results(|events| {
            let events = Footnote::process(events);
            let events = Figure::process(events);
            let events = TypstImage::process(events, slug);
            let events = Embed::process(events);
            normalize_html_content(to_contents(events))
        })
        .wrap_err("failed to parse metadata")?;

    let metadata = HTMLMetaData(metadata);

    Ok(ShallowSection { metadata, content })
}

pub fn parse_spanned_markdown(markdown_input: &str, slug: Slug) -> HTMLContent {
    let events = pulldown_cmark::Parser::new_ext(markdown_input, OPTIONS);
    let events = ignore_paragraph(events);
    let events = Embed::process(TypstImage::process(events, slug));
    normalize_html_content(to_contents(events))
}

fn normalize_html_content(mut content: Vec<LazyContent>) -> HTMLContent {
    if let [LazyContent::Plain(html)] = content.as_mut_slice() {
        HTMLContent::Plain(mem::take(html))
    } else {
        HTMLContent::Lazy(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_td() {
        use crate::ordered_map::OrderedMap;
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
        let mut metadata: OrderedMap<String, HTMLContent> = OrderedMap::new();
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
