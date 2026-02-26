// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{collections::HashMap, collections::HashSet, mem};

use eyre::{eyre, WrapErr};
use itertools::Itertools;
use pulldown_cmark::Options;

use crate::{
    entry::{HTMLMetaData, MetaData, KEY_EXT, KEY_SLUG, KEY_TAXON, KEY_TITLE},
    environment::input_path,
    ordered_map::OrderedMap,
    path_utils,
    process::{
        content::to_contents,
        embed_markdown::Embed,
        figure::Figure,
        footnote::Footnote,
        ignore_paragraph,
        metadata::{self, Metadata},
        text_elaborator::TextElaborator,
        typst_image::TypstImage,
    },
    slug::{self, Slug},
};

use super::{
    section::{LazyContent, SectionOption},
    HTMLContent, UnresolvedSection,
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
    let extracted = extract_subtrees(&source, source_slug)?;

    let mut root = parse_markdown_source(&extracted.root_source, source_slug)
        .wrap_err_with(|| eyre!("failed to parse root markdown section `{source_slug}`"))?;
    patch_root_subtree_embeds(&mut root, &extracted.subtrees)?;

    let mut sections = vec![(source_slug, root)];
    for subtree in extracted.subtrees {
        let mut section =
            parse_markdown_source(&subtree.body, subtree.slug).wrap_err_with(|| {
                eyre!(
                    "failed to parse subtree section `{}` (from `{}`)",
                    subtree.slug,
                    source_slug
                )
            })?;
        apply_subtree_defaults(&mut section, &subtree);
        sections.push((subtree.slug, section));
    }

    ensure_unique_section_slugs(&sections, source_slug)?;
    Ok(sections)
}

fn parse_markdown_source(source: &str, slug: Slug) -> eyre::Result<UnresolvedSection> {
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

const SUBTREE_PLACEHOLDER_PREFIX: &str = "/__kodama_subtree_internal__/";

#[derive(Debug, Clone)]
struct SubtreeSpec {
    tag: String,
    slug: Slug,
    body: String,
    placeholder_url: String,
    option: SectionOption,
    title: Option<String>,
    taxon: Option<String>,
}

#[derive(Debug)]
struct ExtractedSubtrees {
    root_source: String,
    subtrees: Vec<SubtreeSpec>,
}

#[derive(Debug)]
struct OpenTag {
    name: String,
    attrs: String,
    end: usize, // index of `>`
    self_closing: bool,
}

#[derive(Debug)]
struct CloseTag {
    name: String,
    end: usize, // index of `>`
}

#[derive(Debug, Clone, Copy)]
struct CloseTagRange {
    start: usize, // index of `<`
    end: usize,   // index of `>`
}

fn extract_subtrees(source: &str, current_slug: Slug) -> eyre::Result<ExtractedSubtrees> {
    let mut root_source = String::new();
    let mut subtrees = Vec::new();
    let mut cursor = 0;

    while let Some(rel) = source[cursor..].find('<') {
        let lt = cursor + rel;
        root_source.push_str(&source[cursor..lt]);

        let Some(open_tag) = parse_open_tag(source, lt) else {
            root_source.push('<');
            cursor = lt + 1;
            continue;
        };

        if !is_subtree_tag(&open_tag.name) || open_tag.self_closing {
            root_source.push('<');
            cursor = lt + 1;
            continue;
        }

        let attrs = parse_attrs(&open_tag.attrs)?;
        let Some(raw_slug) = attrs.get("slug") else {
            // Anonymous subtree mode: keep the whole block unchanged and do not
            // scan nested tags for subtree extraction.
            if let Some(close_range) =
                find_matching_close_tag(source, open_tag.end + 1, &open_tag.name)
            {
                root_source.push_str(&source[lt..=close_range.end]);
                cursor = close_range.end + 1;
            } else {
                // Fallback for malformed/unclosed anonymous tags: keep open tag unchanged.
                root_source.push_str(&source[lt..=open_tag.end]);
                cursor = open_tag.end + 1;
            }
            continue;
        };
        if raw_slug.trim().is_empty() {
            return Err(eyre!(
                "invalid subtree tag `<{}>` in `{}`: `slug` cannot be empty",
                open_tag.name,
                current_slug
            ));
        }

        let Some(close_range) = find_matching_close_tag(source, open_tag.end + 1, &open_tag.name)
        else {
            return Err(eyre!(
                "unclosed subtree tag `<{} slug=\"...\">` in `{}`",
                open_tag.name,
                current_slug
            ));
        };

        let option = parse_subtree_option(&attrs);
        let slug = resolve_subtree_slug(current_slug, raw_slug).wrap_err_with(|| {
            eyre!(
                "invalid subtree tag `<{} slug=\"{}\">` in `{}`",
                open_tag.name,
                raw_slug,
                current_slug
            )
        })?;
        let placeholder_url = format!("{SUBTREE_PLACEHOLDER_PREFIX}{}", subtrees.len());

        root_source.push_str(&format!("\n[]({placeholder_url}#:embed)\n"));

        let body = source[open_tag.end + 1..close_range.start].to_string();
        let title = attrs.get("title").cloned();
        let taxon = attrs.get("taxon").cloned();

        subtrees.push(SubtreeSpec {
            tag: open_tag.name,
            slug,
            body,
            placeholder_url,
            option,
            title,
            taxon,
        });

        cursor = close_range.end + 1;
    }

    root_source.push_str(&source[cursor..]);
    Ok(ExtractedSubtrees {
        root_source,
        subtrees,
    })
}

fn find_matching_close_tag(
    source: &str,
    mut cursor: usize,
    tag_name: &str,
) -> Option<CloseTagRange> {
    let mut depth = 1usize;

    while let Some(rel) = source[cursor..].find('<') {
        let start = cursor + rel;

        if let Some(open) = parse_open_tag(source, start) {
            if open.name.eq_ignore_ascii_case(tag_name) && !open.self_closing {
                depth += 1;
            }
            cursor = open.end + 1;
            continue;
        }

        if let Some(close) = parse_close_tag(source, start) {
            if close.name.eq_ignore_ascii_case(tag_name) {
                depth -= 1;
                if depth == 0 {
                    return Some(CloseTagRange {
                        start,
                        end: close.end,
                    });
                }
            }
            cursor = close.end + 1;
            continue;
        }

        cursor = start + 1;
    }

    None
}

fn parse_open_tag(source: &str, start: usize) -> Option<OpenTag> {
    let bytes = source.as_bytes();
    if bytes.get(start).copied()? != b'<' {
        return None;
    }
    if bytes.get(start + 1).copied() == Some(b'/') {
        return None;
    }

    let end = find_tag_end(source, start + 1)?;
    let mut inner = source[start + 1..end].trim();
    if inner.is_empty() {
        return None;
    }

    let mut self_closing = false;
    if inner.ends_with('/') {
        self_closing = true;
        inner = inner[..inner.len() - 1].trim_end();
    }

    let (name, attrs) = split_tag_name_and_attrs(inner)?;
    Some(OpenTag {
        name: name.to_ascii_lowercase(),
        attrs: attrs.to_string(),
        end,
        self_closing,
    })
}

fn parse_close_tag(source: &str, start: usize) -> Option<CloseTag> {
    let bytes = source.as_bytes();
    if bytes.get(start).copied()? != b'<' {
        return None;
    }
    if bytes.get(start + 1).copied()? != b'/' {
        return None;
    }

    let end = find_tag_end(source, start + 2)?;
    let inner = source[start + 2..end].trim();
    if !is_valid_tag_name(inner) {
        return None;
    }

    Some(CloseTag {
        name: inner.to_ascii_lowercase(),
        end,
    })
}

fn split_tag_name_and_attrs(inner: &str) -> Option<(&str, &str)> {
    let mut split_at = inner.len();
    for (idx, ch) in inner.char_indices() {
        if ch.is_whitespace() {
            split_at = idx;
            break;
        }
    }
    let name = &inner[..split_at];
    if !is_valid_tag_name(name) {
        return None;
    }
    let attrs = if split_at < inner.len() {
        inner[split_at..].trim()
    } else {
        ""
    };
    Some((name, attrs))
}

fn is_valid_tag_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
}

fn find_tag_end(source: &str, mut idx: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut quote: Option<u8> = None;

    while idx < bytes.len() {
        let b = bytes[idx];
        if let Some(q) = quote {
            if b == q {
                quote = None;
            } else if b == b'\\' {
                idx += 1;
            }
        } else if b == b'"' || b == b'\'' {
            quote = Some(b);
        } else if b == b'>' {
            return Some(idx);
        }
        idx += 1;
    }

    None
}

fn parse_attrs(attrs: &str) -> eyre::Result<HashMap<String, String>> {
    let bytes = attrs.as_bytes();
    let mut i = 0usize;
    let mut parsed = HashMap::new();

    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }

        let key_start = i;
        while i < bytes.len()
            && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
        {
            i += 1;
        }
        if key_start == i {
            return Err(eyre!("malformed subtree tag attribute: `{}`", attrs));
        }
        let key = attrs[key_start..i].to_ascii_lowercase();

        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }

        let mut value = String::new();
        if i < bytes.len() && bytes[i] == b'=' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= bytes.len() {
                return Err(eyre!(
                    "malformed subtree tag attribute `{}` in `{}`: missing value",
                    key,
                    attrs
                ));
            }

            if bytes[i] == b'"' || bytes[i] == b'\'' {
                let quote = bytes[i];
                i += 1;
                let start = i;
                while i < bytes.len() && bytes[i] != quote {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                if i >= bytes.len() {
                    return Err(eyre!("malformed subtree tag attribute: unclosed quote"));
                }
                value = attrs[start..i].to_string();
                i += 1;
            } else {
                let start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                value = attrs[start..i].to_string();
            }
        }

        let value = htmlize::unescape_attribute(&value).into_owned();
        parsed.insert(key, value);
    }

    Ok(parsed)
}

fn parse_subtree_option(attrs: &HashMap<String, String>) -> SectionOption {
    let defaults = SectionOption::default();
    SectionOption::new(
        parse_bool_attr(attrs.get("numbering"), defaults.numbering),
        parse_bool_attr(attrs.get("open"), defaults.details_open),
        parse_bool_attr(attrs.get("catalog"), defaults.catalog),
    )
}

fn parse_bool_attr(value: Option<&String>, default: bool) -> bool {
    match value.map(|s| s.as_str()) {
        None | Some("auto") => default,
        Some("false") | Some("0") | Some("none") => false,
        _ => true,
    }
}

fn resolve_subtree_slug(current_slug: Slug, raw_slug: &str) -> eyre::Result<Slug> {
    let component = raw_slug.trim();
    if component.is_empty() {
        return Err(eyre!("slug cannot be empty"));
    }
    if component == "." || component == ".." {
        return Err(eyre!("slug must be a concrete path component name"));
    }
    if component.contains('/') || component.contains('\\') {
        return Err(eyre!(
            "slug must be a single path component name without separators"
        ));
    }

    // Subtree slugs are always relative to the current section prefix.
    let relative = path_utils::relative_to_current(current_slug.as_str(), component);
    Ok(slug::to_slug(relative))
}

fn is_subtree_tag(tag: &str) -> bool {
    matches!(
        tag,
        "block"
            | "exegesis"
            | "definition"
            | "proposition"
            | "remark"
            | "conjecture"
            | "postulate"
            | "claim"
            | "observation"
            | "fact"
            | "hypothesis"
            | "axiom"
            | "lemma"
            | "theorem"
            | "corollary"
            | "example"
            | "proof"
    )
}

fn ensure_unique_section_slugs(
    sections: &[(Slug, UnresolvedSection)],
    source_slug: Slug,
) -> eyre::Result<()> {
    let mut seen = HashSet::new();
    for (slug, _) in sections {
        if !seen.insert(*slug) {
            return Err(eyre!(
                "duplicate subtree slug `{}` generated from `{}`",
                slug,
                source_slug
            ));
        }
    }
    Ok(())
}

fn patch_root_subtree_embeds(
    root: &mut UnresolvedSection,
    subtrees: &[SubtreeSpec],
) -> eyre::Result<()> {
    if subtrees.is_empty() {
        return Ok(());
    }

    let mut matched = HashSet::new();
    let mut by_placeholder = HashMap::new();
    for subtree in subtrees {
        by_placeholder.insert(subtree.placeholder_url.as_str(), subtree);
    }

    let HTMLContent::Lazy(contents) = &mut root.content else {
        return Err(eyre!(
            "internal subtree parser error: expected lazy root content with embed placeholders"
        ));
    };

    for content in contents {
        let LazyContent::Embed(embed) = content else {
            continue;
        };
        let Some(spec) = by_placeholder.get(embed.url.as_str()) else {
            continue;
        };
        embed.url = format!("/{}", spec.slug);
        embed.option = spec.option.clone();
        if let Some(title) = &spec.title {
            embed.title = Some(title.clone());
        }
        matched.insert(spec.placeholder_url.as_str());
    }

    if matched.len() != subtrees.len() {
        return Err(eyre!(
            "internal subtree parser error: placeholder count mismatch (matched {}, expected {})",
            matched.len(),
            subtrees.len()
        ));
    }

    Ok(())
}

fn apply_subtree_defaults(section: &mut UnresolvedSection, spec: &SubtreeSpec) {
    section.metadata.0.insert(
        KEY_SLUG.to_string(),
        HTMLContent::Plain(spec.slug.to_string()),
    );
    section
        .metadata
        .0
        .insert(KEY_EXT.to_string(), HTMLContent::Plain("md".to_string()));

    if section.metadata.title().is_none() {
        if let Some(title) = &spec.title {
            section
                .metadata
                .0
                .insert(KEY_TITLE.to_string(), HTMLContent::Plain(title.clone()));
        }
    }

    if section.metadata.taxon().is_none() {
        let default_taxon = if spec.tag == "block" {
            None
        } else {
            Some(spec.tag.as_str())
        };
        let taxon = spec
            .taxon
            .as_deref()
            .or(default_taxon)
            .map(metadata::display_taxon);
        if let Some(taxon) = taxon {
            section
                .metadata
                .0
                .insert(KEY_TAXON.to_string(), HTMLContent::Plain(taxon));
        }
    }
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
        let events = TextElaborator::process(events);
        let events = Embed::process(events, mocked_slug);

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
        let events = TextElaborator::process(events);
        let events = Embed::process(events, mocked_slug);

        let content = normalize_html_content(to_contents(events));
        assert_eq!(
            content.as_str().unwrap(),
            "<pre><code class=\"language-rs\">let x = 1;\n</code></pre>\n"
        );
    }

    #[test]
    pub fn test_reference_link() {
        let source =
            "---\nlink: [Alice][example]\n---\n\n[Bob][example]\n\n[example]: https://example.com";
        let mocked_slug = Slug::new("-");

        let events = pulldown_cmark::Parser::new_ext(source, OPTIONS);
        let events = Footnote::process(events, mocked_slug);
        let events = Figure::process(events);
        let events = TypstImage::process(events, mocked_slug);
        let events = TextElaborator::process(events);
        let events = Embed::process(events, mocked_slug);

        let content = normalize_html_content(to_contents(events));
        assert_eq!(content.as_str().unwrap(), "<p><span class=\"link external\"><a href=\"https://example.com\" title=\"Bob [https://example.com]\">Bob</a></span></p>\n");
    }

    #[test]
    pub fn test_parse_spanned_markdown_wraps_cjk_text() {
        let content = parse_spanned_markdown("hello \u{4e2d}\u{6587} world", Slug::new("-"));
        assert_eq!(
            content.as_str().unwrap(),
            "hello <span lang=\"zh\">\u{4e2d}\u{6587}</span> world"
        );
    }

    #[test]
    fn test_extract_subtrees_rewrites_root_to_embed_placeholders() {
        let source = "before\n<remark slug=\"child\" title=\"Child\">\nhello\n</remark>\nafter";
        let extracted = extract_subtrees(source, Slug::new("doc/index")).unwrap();

        assert!(extracted
            .root_source
            .contains("[](/__kodama_subtree_internal__/0#:embed)"));
        assert_eq!(extracted.subtrees.len(), 1);
        assert_eq!(extracted.subtrees[0].slug, Slug::new("doc/child"));
        assert_eq!(extracted.subtrees[0].title.as_deref(), Some("Child"));
        assert_eq!(extracted.subtrees[0].tag, "remark");
    }

    #[test]
    fn test_extract_subtrees_ignores_tags_without_slug_attribute() {
        let source = "<remark>plain</remark>";
        let extracted = extract_subtrees(source, Slug::new("index")).unwrap();
        assert_eq!(extracted.subtrees.len(), 0);
        assert_eq!(extracted.root_source, source);
    }

    #[test]
    fn test_extract_subtrees_keeps_anonymous_wrapper_without_extracting_nested_slug_blocks() {
        let source = r#"<remark>
<proof slug="child">inner</proof>
</remark>"#;
        let extracted = extract_subtrees(source, Slug::new("index")).unwrap();
        assert_eq!(extracted.subtrees.len(), 0);
        assert_eq!(extracted.root_source, source);
    }

    #[test]
    fn test_parse_markdown_sections_anonymous_subtree_emits_no_independent_child_section() {
        let source = r#"
<remark>
anonymous body
</remark>
"#;
        let extracted = extract_subtrees(source, Slug::new("index")).unwrap();
        let mut sections = Vec::new();
        let mut root = parse_markdown_source(&extracted.root_source, Slug::new("index")).unwrap();
        patch_root_subtree_embeds(&mut root, &extracted.subtrees).unwrap();
        sections.push((Slug::new("index"), root));

        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].0, Slug::new("index"));
        assert!(sections[0]
            .1
            .metadata
            .slug()
            .is_some_and(|slug| slug == Slug::new("index")));
    }

    #[test]
    fn test_parse_markdown_sections_yields_root_and_child_sections() {
        let source = r#"
---
title: Root
---
<remark slug="child">
---
title: Child
---
child body
</remark>
"#;
        let extracted = extract_subtrees(source, Slug::new("book/index")).unwrap();
        let mut root =
            parse_markdown_source(&extracted.root_source, Slug::new("book/index")).unwrap();
        patch_root_subtree_embeds(&mut root, &extracted.subtrees).unwrap();

        let HTMLContent::Lazy(contents) = &root.content else {
            panic!("expected lazy root content");
        };
        let embeds: Vec<_> = contents
            .iter()
            .filter_map(|content| match content {
                LazyContent::Embed(embed) => Some(embed.url.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(embeds, vec!["/book/child".to_string()]);

        let mut child =
            parse_markdown_source(&extracted.subtrees[0].body, extracted.subtrees[0].slug).unwrap();
        apply_subtree_defaults(&mut child, &extracted.subtrees[0]);
        assert_eq!(child.metadata.slug(), Some(Slug::new("book/child")));
        assert_eq!(
            child
                .metadata
                .title()
                .and_then(|v| v.as_string())
                .map(String::as_str),
            Some("Child")
        );
    }

    #[test]
    fn test_extract_subtrees_rejects_relative_prefix_slug() {
        let source = r#"<remark slug="./child">x</remark>"#;
        let err = extract_subtrees(source, Slug::new("doc/index")).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("invalid subtree tag"));
        assert!(msg.contains(r#"slug="./child""#));
    }

    #[test]
    fn test_extract_subtrees_rejects_absolute_or_nested_slug() {
        let absolute = r#"<remark slug="/child">x</remark>"#;
        let nested = r#"<remark slug="a/b">x</remark>"#;

        let absolute_err = extract_subtrees(absolute, Slug::new("doc/index")).unwrap_err();
        let nested_err = extract_subtrees(nested, Slug::new("doc/index")).unwrap_err();
        assert!(absolute_err.to_string().contains(r#"slug="/child""#));
        assert!(nested_err.to_string().contains(r#"slug="a/b""#));
    }
}
