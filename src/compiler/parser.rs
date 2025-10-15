// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::mem;

use eyre::{eyre, WrapErr};
use itertools::Itertools;
use pulldown_cmark::Options;

use crate::{
    entry::{HTMLMetaData, KEY_EXT, KEY_SLUG},
    environment::input_path,
    ordered_map::OrderedMap,
    process::{
        content::to_contents, embed_markdown::Embed, figure::Figure, footnote::Footnote,
        ignore_paragraph, metadata::Metadata, typst_image::TypstImage,
    },
    slug::Slug,
};

use super::{section::LazyContent, HTMLContent, ShallowSection};

pub const OPTIONS: Options = Options::ENABLE_MATH
    .union(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS)
    .union(Options::ENABLE_TABLES)
    .union(Options::ENABLE_SMART_PUNCTUATION)
    .union(Options::ENABLE_FOOTNOTES);

/// For Typst cases, see [`crate::compiler::typst::parse_typst`]
pub fn initialize(slug: Slug) -> eyre::Result<(String, OrderedMap<String, HTMLContent>)> {
    let mut metadata: OrderedMap<String, HTMLContent> = OrderedMap::new();
    let fullname = format!("{}.md", slug);
    metadata.insert(KEY_SLUG.to_string(), HTMLContent::Plain(slug.to_string()));
    metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain("md".to_string()));

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
            let events = Footnote::process(events, slug);
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
pub mod tests {
    use super::*;

    #[test]
    pub fn test_table_td() {
        let source = "| a | b |\n| - | - |\n| c | d |";
        let mocked_slug = Slug::new("-");

        let events = pulldown_cmark::Parser::new_ext(source, OPTIONS);
        let events = Footnote::process(events, mocked_slug);
        let events = Figure::process(events);
        let events = TypstImage::process(events, mocked_slug);
        let events = Embed::process(events);

        let content = normalize_html_content(to_contents(events));
        assert_eq!(content.as_str().unwrap(), "<table><thead><tr><th>a</th><th>b</th></tr></thead><tbody>\n<tr><td>c</td><td>d</td></tr>\n</tbody></table>\n");
    }

    #[test]
    pub fn test_code_block() {
        let source = "```rs\nlet x = 1;\n```";
        let mocked_slug = Slug::new("-");

        let events = pulldown_cmark::Parser::new_ext(source, OPTIONS);
        let events = Footnote::process(events, mocked_slug);
        let events = Figure::process(events);
        let events = TypstImage::process(events, mocked_slug);
        let events = Embed::process(events);

        let content = normalize_html_content(to_contents(events));
        assert_eq!(content.as_str().unwrap(), "<pre><code class=\"language-rs\">let x = 1;\n</code></pre>\n");
    }

    #[test]
    pub fn test_reference_link() {
        let source = "---\nlink: [Alice][example]\n---\n\n[Bob][example]\n\n[example]: https://example.com";
        let mocked_slug = Slug::new("-");

        let events = pulldown_cmark::Parser::new_ext(source, OPTIONS);
        let events = Footnote::process(events, mocked_slug);
        let events = Figure::process(events);
        let events = TypstImage::process(events, mocked_slug);
        let events = Embed::process(events);

        let content = normalize_html_content(to_contents(events));
        assert_eq!(content.as_str().unwrap(), "<p><span class=\"link external\"><a href=\"https://example.com\" title=\"Bob [https://example.com]\">Bob</a></span></p>\n");
    }
}
