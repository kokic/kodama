// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Alias Qli (@AliasQli), Spore (@s-cerevisiae), Kokic (@kokic)

use camino::Utf8Path;
use eyre::{eyre, WrapErr};

use super::html_parser::{HTMLParser, HTMLTagKind};
use super::section::{EmbedContent, LocalLink, SectionOption};
use super::section::{HTMLContent, HTMLContentBuilder, LazyContent};
use super::UnresolvedSection;
use crate::{
    entry::{
        HTMLMetaData, KEY_EXT, KEY_INTERNAL_ANON_SUBTREE, KEY_SLUG, KEY_SOURCE_SLUG, KEY_TAXON,
        KEY_TITLE,
    },
    ordered_map::OrderedMap,
    path_utils,
    process::metadata,
    slug::{self, Slug},
    typst_cli,
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    str,
};

const ANON_SUBTREE_SLUG_PREFIX: &str = "__kodama_anon_subtree_internal__";

#[derive(Default)]
struct AnonymousSubtreeSlugState {
    used_slugs: HashSet<Slug>,
    anonymous_ordinals: HashMap<Slug, usize>,
}

impl AnonymousSubtreeSlugState {
    fn claim_slug(&mut self, slug: Slug, source_slug: Slug) -> eyre::Result<()> {
        if self.used_slugs.insert(slug) {
            Ok(())
        } else {
            Err(eyre!(
                "duplicate typst subtree slug `{}` generated from `{}`",
                slug,
                source_slug
            ))
        }
    }

    fn allocate_anonymous_slug(&mut self, base_slug: Slug) -> Slug {
        let ordinal = self.anonymous_ordinals.entry(base_slug).or_insert(0);
        loop {
            let candidate = anonymous_slug_for(base_slug, *ordinal);
            *ordinal += 1;
            if self.used_slugs.insert(candidate) {
                return candidate;
            }
        }
    }
}

fn parse_bool(m: Option<&Cow<'_, str>>, def: bool) -> bool {
    match m.map(|s| s.as_ref()) {
        None | Some("auto") => def,
        Some("false") | Some("0") | Some("none") => false,
        _ => true,
    }
}

fn parse_typst_html(
    html_str: &str,
    source_slug: Slug,
    current_slug: Slug,
    metadata: &mut OrderedMap<String, HTMLContent>,
    subtree_sections: &mut Vec<(Slug, UnresolvedSection)>,
    slug_state: &mut AnonymousSubtreeSlugState,
    allow_subtree: bool,
) -> eyre::Result<HTMLContent> {
    let mut builder = HTMLContentBuilder::new();
    let mut cursor: usize = 0;

    for span in HTMLParser::new(html_str) {
        let span = span.wrap_err("failed to parse kodama tag from typst html")?;

        builder.push_str(&html_str[cursor..span.start]);
        cursor = span.end;

        let attr = |attr_name: &str| {
            span.attrs
                .get(attr_name)
                .ok_or_else(|| eyre!("missing attribute `{attr_name}` in kodama tag"))
        };

        let value = || {
            let value = span
                .attrs
                .get("value")
                .map_or_else(|| span.body.to_string(), |s| s.to_string());
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        };
        match span.kind {
            HTMLTagKind::Meta => {
                let key = attr("key")?.as_ref();
                let mut val = if let Some(value) = span.attrs.get("value") {
                    HTMLContent::Plain(value.to_string())
                } else {
                    parse_typst_html(
                        span.body,
                        source_slug,
                        current_slug,
                        &mut OrderedMap::new(),
                        subtree_sections,
                        slug_state,
                        false,
                    )?
                };
                if key == "taxon" {
                    if let HTMLContent::Plain(v) = val {
                        val = HTMLContent::Plain(metadata::display_taxon(&v));
                    }
                }
                metadata.insert(key.to_string(), val);
            }
            HTMLTagKind::Embed => {
                let def = SectionOption::default();

                let url = attr("url")?.to_string();
                let title = value();
                let numbering = parse_bool(span.attrs.get("numbering"), def.numbering);
                let details_open = parse_bool(span.attrs.get("open"), def.details_open);
                let catalog = parse_bool(span.attrs.get("catalog"), def.catalog);
                builder.push(LazyContent::Embed(EmbedContent {
                    url,
                    title,
                    option: SectionOption::new(numbering, details_open, catalog),
                }))
            }
            HTMLTagKind::Local { span: _ } => {
                let url = attr(KEY_SLUG)?.to_string();
                let text = value();
                builder.push(LazyContent::Local(LocalLink { url, text }))
            }
            HTMLTagKind::Subtree => {
                if !allow_subtree {
                    return Err(eyre!(
                        "typst subtree tag is not allowed in metadata value while parsing `{}`",
                        source_slug
                    ));
                }

                let (subtree_slug, anonymous) = if let Some(raw_slug) = span.attrs.get("slug") {
                    let raw_slug = raw_slug.as_ref();
                    let subtree_slug =
                        resolve_subtree_slug(current_slug, raw_slug).wrap_err_with(|| {
                            eyre!(
                                "invalid typst subtree slug `{}` in `{}`",
                                raw_slug,
                                source_slug
                            )
                        })?;
                    slug_state.claim_slug(subtree_slug, source_slug)?;
                    (subtree_slug, false)
                } else {
                    (slug_state.allocate_anonymous_slug(current_slug), true)
                };

                let def = SectionOption::default();
                let numbering = parse_bool(span.attrs.get("numbering"), def.numbering);
                let details_open = parse_bool(span.attrs.get("open"), def.details_open);
                let catalog = parse_bool(span.attrs.get("catalog"), def.catalog);
                let option = SectionOption::new(numbering, details_open, catalog);

                let title = span
                    .attrs
                    .get("title")
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty());
                let taxon = span
                    .attrs
                    .get("taxon")
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty());

                builder.push(LazyContent::Embed(EmbedContent {
                    url: format!("/{subtree_slug}"),
                    title: title.clone(),
                    option,
                }));

                let mut subtree_metadata = OrderedMap::new();
                subtree_metadata.insert(
                    KEY_SLUG.to_string(),
                    HTMLContent::Plain(subtree_slug.to_string()),
                );
                subtree_metadata
                    .insert(KEY_EXT.to_string(), HTMLContent::Plain("typst".to_string()));
                subtree_metadata.insert(
                    KEY_SOURCE_SLUG.to_string(),
                    HTMLContent::Plain(source_slug.to_string()),
                );
                if anonymous {
                    subtree_metadata.insert(
                        KEY_INTERNAL_ANON_SUBTREE.to_string(),
                        HTMLContent::Plain("true".to_string()),
                    );
                }
                let nested_current_slug = if anonymous {
                    current_slug
                } else {
                    subtree_slug
                };
                let subtree_content = parse_typst_html(
                    span.body,
                    source_slug,
                    nested_current_slug,
                    &mut subtree_metadata,
                    subtree_sections,
                    slug_state,
                    true,
                )
                .wrap_err_with(|| {
                    eyre!(
                        "failed to parse typst subtree section `{}` in `{}`",
                        subtree_slug,
                        source_slug
                    )
                })?;
                apply_subtree_defaults(&mut subtree_metadata, title.as_deref(), taxon.as_deref());
                subtree_sections.push((
                    subtree_slug,
                    UnresolvedSection {
                        metadata: HTMLMetaData(subtree_metadata),
                        content: subtree_content,
                    },
                ));
            }
        }
    }

    builder.push_str(&html_str[cursor..]);

    Ok(builder.build())
}

fn apply_subtree_defaults(
    metadata: &mut OrderedMap<String, HTMLContent>,
    title: Option<&str>,
    taxon: Option<&str>,
) {
    if !metadata.contains_key(KEY_TITLE) {
        if let Some(title) = title {
            metadata.insert(KEY_TITLE.to_string(), HTMLContent::Plain(title.to_string()));
        }
    }
    if !metadata.contains_key(KEY_TAXON) {
        if let Some(taxon) = taxon {
            metadata.insert(
                KEY_TAXON.to_string(),
                HTMLContent::Plain(metadata::display_taxon(taxon)),
            );
        }
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
    let relative = path_utils::relative_to_current(current_slug.as_str(), component);
    Ok(slug::to_slug(relative))
}

fn anonymous_slug_for(base_slug: Slug, ordinal: usize) -> Slug {
    let disambiguator = slug::to_hash_id(base_slug.as_str());
    let component = format!("{ANON_SUBTREE_SLUG_PREFIX}-{disambiguator}-{ordinal}");
    let relative = path_utils::relative_to_current(base_slug.as_str(), component);
    slug::to_slug(relative)
}

fn ensure_unique_section_slugs(
    sections: &[(Slug, UnresolvedSection)],
    source_slug: Slug,
) -> eyre::Result<()> {
    let mut seen = HashSet::new();
    for (slug, _) in sections {
        if !seen.insert(*slug) {
            return Err(eyre!(
                "duplicate typst subtree slug `{}` generated from `{}`",
                slug,
                source_slug
            ));
        }
    }
    Ok(())
}

fn parse_typst_sections_from_html(
    source_slug: Slug,
    html_str: &str,
) -> eyre::Result<Vec<(Slug, UnresolvedSection)>> {
    let mut metadata: OrderedMap<String, HTMLContent> = OrderedMap::new();
    metadata.insert(
        KEY_SLUG.to_string(),
        HTMLContent::Plain(source_slug.to_string()),
    );
    metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain("typst".to_string()));
    metadata.insert(
        KEY_SOURCE_SLUG.to_string(),
        HTMLContent::Plain(source_slug.to_string()),
    );

    let mut slug_state = AnonymousSubtreeSlugState::default();
    slug_state.claim_slug(source_slug, source_slug)?;
    let mut subtree_sections = Vec::new();
    let content = parse_typst_html(
        html_str,
        source_slug,
        source_slug,
        &mut metadata,
        &mut subtree_sections,
        &mut slug_state,
        true,
    )?;

    let mut sections = vec![(
        source_slug,
        UnresolvedSection {
            metadata: HTMLMetaData(metadata),
            content,
        },
    )];
    sections.extend(subtree_sections);
    ensure_unique_section_slugs(&sections, source_slug)?;
    Ok(sections)
}

pub fn parse_typst_sections<P: AsRef<Utf8Path>>(
    slug: Slug,
    root_dir: P,
) -> eyre::Result<Vec<(Slug, UnresolvedSection)>> {
    let typst_root_dir = root_dir.as_ref();
    let relative_path = format!("{}.typst", slug);
    let html_str = typst_cli::file_to_html(&relative_path, typst_root_dir.as_ref())
        .wrap_err_with(|| eyre!("failed to compile typst file `{relative_path}` to html"))?;

    parse_typst_sections_from_html(slug, &html_str)
        .wrap_err_with(|| eyre!("failed to parse typst html structure in `{relative_path}`"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compiler::section::LazyContent,
        entry::{MetaData, KEY_INTERNAL_ANON_SUBTREE},
    };

    fn find_section(sections: &[(Slug, UnresolvedSection)], slug: Slug) -> &UnresolvedSection {
        sections
            .iter()
            .find_map(|(s, section)| (*s == slug).then_some(section))
            .expect("expected section")
    }

    #[test]
    fn test_parse_typst_sections_extracts_named_subtree() {
        let html = r#"
<p>root</p>
<kodama-subtree slug="child" title="Child" numbering="true"><p>child</p></kodama-subtree>
"#;
        let sections = parse_typst_sections_from_html(Slug::new("book/index"), html).unwrap();
        assert_eq!(sections.len(), 2);

        let root = find_section(&sections, Slug::new("book/index"));
        let root_contents = match &root.content {
            HTMLContent::Lazy(contents) => contents,
            _ => panic!("expected lazy root content"),
        };
        let embed = root_contents
            .iter()
            .find_map(|content| match content {
                LazyContent::Embed(embed) => Some(embed),
                _ => None,
            })
            .expect("expected subtree embed");
        assert_eq!(embed.url, "/book/child");
        assert_eq!(embed.title.as_deref(), Some("Child"));
        assert!(embed.option.numbering);

        let child = find_section(&sections, Slug::new("book/child"));
        assert_eq!(
            child
                .metadata
                .title()
                .and_then(HTMLContent::as_string)
                .map(String::as_str),
            Some("Child")
        );
        assert_eq!(child.metadata.ext().map(String::as_str), Some("typst"));
        assert_eq!(
            child.metadata.get_str(KEY_SOURCE_SLUG).map(String::as_str),
            Some("book/index")
        );
    }

    #[test]
    fn test_parse_typst_sections_subtree_body_metadata_overrides_attr_defaults() {
        let html = r#"
<kodama-subtree slug="child" title="Outer">
<kodama-meta key="title" value="Inner"></kodama-meta>
<p>child</p>
</kodama-subtree>
"#;
        let sections = parse_typst_sections_from_html(Slug::new("book/index"), html).unwrap();
        let root = find_section(&sections, Slug::new("book/index"));
        let root_contents = match &root.content {
            HTMLContent::Lazy(contents) => contents,
            _ => panic!("expected lazy root content"),
        };
        let embed = root_contents
            .iter()
            .find_map(|content| match content {
                LazyContent::Embed(embed) => Some(embed),
                _ => None,
            })
            .expect("expected subtree embed");
        assert_eq!(embed.title.as_deref(), Some("Outer"));

        let child = find_section(&sections, Slug::new("book/child"));
        assert_eq!(
            child
                .metadata
                .title()
                .and_then(HTMLContent::as_string)
                .map(String::as_str),
            Some("Inner")
        );
    }

    #[test]
    fn test_parse_typst_sections_extracts_anonymous_subtree() {
        let html = r#"
<p>root</p>
<kodama-subtree title="Anonymous"><p>child</p></kodama-subtree>
"#;
        let sections = parse_typst_sections_from_html(Slug::new("book/index"), html).unwrap();
        assert_eq!(sections.len(), 2);

        let root = find_section(&sections, Slug::new("book/index"));
        let root_contents = match &root.content {
            HTMLContent::Lazy(contents) => contents,
            _ => panic!("expected lazy root content"),
        };
        let embed = root_contents
            .iter()
            .find_map(|content| match content {
                LazyContent::Embed(embed) => Some(embed),
                _ => None,
            })
            .expect("expected subtree embed");
        assert_eq!(
            embed.url,
            "/book/__kodama_anon_subtree_internal__-book-index-0"
        );
        assert_eq!(embed.title.as_deref(), Some("Anonymous"));

        let anonymous = find_section(
            &sections,
            Slug::new("book/__kodama_anon_subtree_internal__-book-index-0"),
        );
        assert_eq!(
            anonymous
                .metadata
                .get_str(KEY_INTERNAL_ANON_SUBTREE)
                .map(String::as_str),
            Some("true")
        );
    }

    #[test]
    fn test_parse_typst_sections_nested_named_subtree_under_anonymous_wrapper_uses_visible_prefix()
    {
        let html = r#"
<kodama-subtree>
  <kodama-subtree slug="child"><p>nested</p></kodama-subtree>
</kodama-subtree>
"#;
        let sections = parse_typst_sections_from_html(Slug::new("book/index"), html).unwrap();
        assert!(sections
            .iter()
            .any(|(slug, _)| *slug == Slug::new("book/child")));

        let anonymous = sections
            .iter()
            .find_map(|(_, section)| {
                section
                    .metadata
                    .get_str(KEY_INTERNAL_ANON_SUBTREE)
                    .is_some_and(|value| value == "true")
                    .then_some(section)
            })
            .expect("expected anonymous wrapper section");
        let HTMLContent::Lazy(contents) = &anonymous.content else {
            panic!("expected lazy anonymous content");
        };
        let nested_embed_urls: Vec<_> = contents
            .iter()
            .filter_map(|content| match content {
                LazyContent::Embed(embed) => Some(embed.url.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(nested_embed_urls, vec!["/book/child".to_string()]);
    }
}
