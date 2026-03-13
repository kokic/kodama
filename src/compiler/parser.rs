// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{
    collections::{HashSet, VecDeque},
    mem,
};

use eyre::{eyre, WrapErr};
use itertools::Itertools;
use pulldown_cmark::Options;

use crate::{
    entry::{HTMLMetaData, KEY_EXT, KEY_SLUG},
    environment::input_path,
    ordered_map::OrderedMap,
    process::{
        content::to_contents, embed_markdown::Embed, figure::Figure, footnote::Footnote,
        ignore_paragraph, metadata::Metadata, text_elaborator::TextElaborator,
        typst_image::TypstImage,
    },
    slug::Slug,
};

use super::anonymous_slug::AnonymousSlugState;
use super::subtree_slug::ensure_unique_section_slugs;
use super::{section::LazyContent, HTMLContent, UnresolvedSection};

mod subtree;
use subtree::{
    apply_subtree_defaults, compose_subtree_source, extract_shared_reference_definitions,
    extract_subtrees_nested, extract_subtrees_root, patch_root_subtree_embeds, SubtreeSpec,
};

pub const OPTIONS: Options = Options::ENABLE_MATH
    .union(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS)
    .union(Options::ENABLE_TABLES)
    .union(Options::ENABLE_SMART_PUNCTUATION)
    .union(Options::ENABLE_FOOTNOTES)
    .union(Options::ENABLE_GFM)
    .union(Options::ENABLE_STRIKETHROUGH)
    .union(Options::ENABLE_TASKLISTS)
    .union(Options::ENABLE_DEFINITION_LIST)
    .union(Options::ENABLE_HEADING_ATTRIBUTES);

/// For Typst cases, see [`crate::compiler::typst::parse_typst`]
pub fn initialize(slug: Slug) -> eyre::Result<String> {
    let fullname = format!("{}.md", slug);
    let markdown_path = input_path(&fullname);
    std::fs::read_to_string(&markdown_path)
        .wrap_err_with(|| eyre!("failed to read markdown file `{:?}`", markdown_path))
}

pub fn parse_markdown_sections(source_slug: Slug) -> eyre::Result<Vec<(Slug, UnresolvedSection)>> {
    let source = initialize(source_slug)?;
    parse_markdown_sections_from_source(&source, source_slug)
}

pub(super) fn parse_markdown_sections_from_source(
    source: &str,
    source_slug: Slug,
) -> eyre::Result<Vec<(Slug, UnresolvedSection)>> {
    let extracted = extract_subtrees_root(source, source_slug)?;
    let shared_reference_definitions = extract_shared_reference_definitions(&extracted.root_source);

    let mut root = parse_markdown_source(&extracted.root_source, source_slug)
        .wrap_err_with(|| eyre!("failed to parse root markdown section `{source_slug}`"))?;
    patch_root_subtree_embeds(&mut root, &extracted.subtrees)?;

    let mut sections = vec![(source_slug, root)];
    let mut used_slugs = HashSet::from([source_slug]);
    let mut pending: VecDeque<(SubtreeSpec, Slug)> = VecDeque::new();
    let mut anonymous_slugs = AnonymousSlugState::default();

    for subtree in extracted.subtrees {
        used_slugs.insert(subtree.slug);
        let nested_base_slug = if subtree.anonymous {
            source_slug
        } else {
            subtree.slug
        };
        pending.push_back((subtree, nested_base_slug));
    }

    while let Some((subtree, nested_base_slug)) = pending.pop_front() {
        let extracted_nested =
            extract_subtrees_nested(&subtree.body, nested_base_slug, subtree.source_slug)?;
        let mut nested_subtrees = extracted_nested.subtrees;
        for nested in &mut nested_subtrees {
            nested.source_slug = subtree.source_slug;
            if nested.anonymous && used_slugs.contains(&nested.slug) {
                nested.slug =
                    anonymous_slugs.allocate_with_used(nested.source_slug, &mut used_slugs);
            } else {
                used_slugs.insert(nested.slug);
            }
        }
        let subtree_source =
            compose_subtree_source(&extracted_nested.root_source, &shared_reference_definitions);
        let mut section =
            parse_markdown_source(&subtree_source, subtree.slug).wrap_err_with(|| {
                eyre!(
                    "failed to parse subtree section `{}` (from `{}`)",
                    subtree.slug,
                    source_slug
                )
            })?;
        patch_root_subtree_embeds(&mut section, &nested_subtrees)?;
        apply_subtree_defaults(&mut section, &subtree);

        for nested in nested_subtrees {
            let next_base_slug = if nested.anonymous {
                nested_base_slug
            } else {
                nested.slug
            };
            pending.push_back((nested, next_base_slug));
        }

        sections.push((subtree.slug, section));
    }

    ensure_unique_section_slugs(&sections, source_slug, "subtree")?;
    Ok(sections)
}

pub(super) fn parse_markdown_source(source: &str, slug: Slug) -> eyre::Result<UnresolvedSection> {
    let mut metadata: OrderedMap<String, HTMLContent> = OrderedMap::new();
    metadata.insert(KEY_SLUG.to_string(), HTMLContent::Plain(slug.to_string()));
    metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain("md".to_string()));

    let events = pulldown_cmark::Parser::new_ext(source, OPTIONS);

    let content = Metadata::process(events, &mut metadata)
        .process_results(|events| {
            let events = Footnote::process(events, slug);
            let events = Figure::process(events);
            let events = TypstImage::process(events, slug);
            let events = TextElaborator::process(events);
            let events = Embed::process(events, slug);
            normalize_html_content(to_contents(events))
        })
        .wrap_err("failed to parse metadata")?;

    let metadata = HTMLMetaData(metadata);
    Ok(UnresolvedSection { metadata, content })
}

pub fn parse_spanned_markdown(markdown_input: &str, slug: Slug) -> HTMLContent {
    let events = pulldown_cmark::Parser::new_ext(markdown_input, OPTIONS);
    let events = ignore_paragraph(events);
    let events = TypstImage::process(events, slug);
    let events = TextElaborator::process(events);
    let events = Embed::process(events, slug);
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
mod tests;
