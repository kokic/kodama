use std::collections::{HashMap, HashSet};

use eyre::{eyre, WrapErr};
use itertools::Itertools;

use crate::{
    entry::{
        MetaData, KEY_EXT, KEY_INTERNAL_ANON_SUBTREE, KEY_SLUG, KEY_SOURCE_POS, KEY_SOURCE_SLUG,
        KEY_TAXON, KEY_TITLE,
    },
    process::metadata,
    slug::Slug,
};

use crate::compiler::{
    anonymous_slug::anonymous_slug_for,
    section::{LazyContent, SectionOption},
    subtree_slug::resolve_subtree_slug,
    HTMLContent, UnresolvedSection,
};

const SUBTREE_PLACEHOLDER_PREFIX: &str = "/__kodama_subtree_internal__/";

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

#[derive(Debug, Clone)]
pub(super) struct SubtreeSpec {
    pub(super) tag: String,
    pub(super) slug: Slug,
    pub(super) body: String,
    pub(super) placeholder_url: String,
    pub(super) option: SectionOption,
    pub(super) title: Option<String>,
    pub(super) taxon: Option<String>,
    pub(super) anonymous: bool,
    pub(super) source_slug: Slug,
    pub(super) source_pos: String,
}

#[derive(Debug)]
pub(super) struct ExtractedSubtrees {
    pub(super) root_source: String,
    pub(super) subtrees: Vec<SubtreeSpec>,
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

pub(super) fn compose_subtree_source(body: &str, shared_reference_definitions: &str) -> String {
    if shared_reference_definitions.trim().is_empty() {
        return body.to_string();
    }
    let mut composed = String::with_capacity(body.len() + shared_reference_definitions.len() + 3);
    composed.push_str(body.trim_end());
    composed.push_str("\n\n");
    composed.push_str(shared_reference_definitions.trim());
    composed.push('\n');
    composed
}

pub(super) fn extract_shared_reference_definitions(root_source: &str) -> String {
    let mut seen_labels = HashSet::new();
    let mut definitions = Vec::new();
    for (label, block) in collect_reference_definition_blocks(root_source) {
        if seen_labels.insert(label) {
            definitions.push(block);
        }
    }

    if definitions.is_empty() {
        String::new()
    } else {
        let mut shared = definitions.join("\n");
        shared.push('\n');
        shared
    }
}

fn collect_reference_definition_blocks(source: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut lines = source.lines().peekable();
    let mut in_fence: Option<char> = None;

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        if let Some(marker) = fenced_code_marker(trimmed) {
            if in_fence == Some(marker) {
                in_fence = None;
            } else if in_fence.is_none() {
                in_fence = Some(marker);
            }
            continue;
        }
        if in_fence.is_some() {
            continue;
        }

        let Some(label) = parse_reference_definition_label(line) else {
            continue;
        };

        let mut block = line.to_string();
        while let Some(next) = lines.peek().copied() {
            if !is_reference_definition_continuation(next) {
                break;
            }
            block.push('\n');
            block.push_str(next);
            lines.next();
        }

        blocks.push((label, block));
    }

    blocks
}

fn parse_reference_definition_label(line: &str) -> Option<String> {
    let leading_spaces = line.chars().take_while(|c| *c == ' ').count();
    if leading_spaces > 3 {
        return None;
    }
    let trimmed = &line[leading_spaces..];
    if !trimmed.starts_with('[') {
        return None;
    }
    let close = trimmed.find("]:")?;
    let raw_label = trimmed.get(1..close)?.trim();
    if raw_label.is_empty() {
        return None;
    }
    Some(normalize_reference_label(raw_label))
}

fn normalize_reference_label(label: &str) -> String {
    label.split_whitespace().join(" ").to_ascii_lowercase()
}

fn is_reference_definition_continuation(line: &str) -> bool {
    !line.trim().is_empty() && (line.starts_with(' ') || line.starts_with('\t'))
}

fn fenced_code_marker(line: &str) -> Option<char> {
    let mut chars = line.chars();
    let marker = chars.next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let count = 1 + chars.take_while(|ch| *ch == marker).count();
    (count >= 3).then_some(marker)
}

pub(super) fn extract_subtrees_root(
    source: &str,
    source_slug: Slug,
) -> eyre::Result<ExtractedSubtrees> {
    extract_subtrees_nested(source, source_slug, source_slug)
}

pub(super) fn extract_subtrees_nested(
    source: &str,
    current_slug: Slug,
    source_slug: Slug,
) -> eyre::Result<ExtractedSubtrees> {
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

        let Some(close_range) = find_matching_close_tag(source, open_tag.end + 1, &open_tag.name)
        else {
            return Err(eyre!(
                "unclosed subtree tag `<{} slug=\"...\">` in `{}`",
                open_tag.name,
                current_slug
            ));
        };

        let option = parse_subtree_option(&attrs);
        let (slug, anonymous) = if let Some(raw_slug) = attrs.get("slug") {
            if raw_slug.trim().is_empty() {
                return Err(eyre!(
                    "invalid subtree tag `<{}>` in `{}`: `slug` cannot be empty",
                    open_tag.name,
                    current_slug
                ));
            }
            (
                resolve_subtree_slug(current_slug, raw_slug).wrap_err_with(|| {
                    eyre!(
                        "invalid subtree tag `<{} slug=\"{}\">` in `{}`",
                        open_tag.name,
                        raw_slug,
                        current_slug
                    )
                })?,
                false,
            )
        } else {
            (anonymous_slug_for(source_slug, subtrees.len()), true)
        };
        let (line, col) = byte_index_to_line_col(source, lt);
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
            anonymous,
            source_slug,
            source_pos: format!("{line}:{col}"),
        });

        cursor = close_range.end + 1;
    }

    root_source.push_str(&source[cursor..]);
    Ok(ExtractedSubtrees {
        root_source,
        subtrees,
    })
}

fn byte_index_to_line_col(source: &str, idx: usize) -> (usize, usize) {
    let idx = idx.min(source.len());
    let mut line = 1usize;
    let mut col = 1usize;

    for ch in source[..idx].chars() {
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
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

pub(super) fn patch_root_subtree_embeds(
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

pub(super) fn apply_subtree_defaults(section: &mut UnresolvedSection, spec: &SubtreeSpec) {
    section.metadata.0.insert(
        KEY_SLUG.to_string(),
        HTMLContent::Plain(spec.slug.to_string()),
    );
    section
        .metadata
        .0
        .insert(KEY_EXT.to_string(), HTMLContent::Plain("md".to_string()));
    section.metadata.0.insert(
        KEY_SOURCE_SLUG.to_string(),
        HTMLContent::Plain(spec.source_slug.to_string()),
    );
    section.metadata.0.insert(
        KEY_SOURCE_POS.to_string(),
        HTMLContent::Plain(spec.source_pos.clone()),
    );
    if spec.anonymous {
        section.metadata.0.insert(
            KEY_INTERNAL_ANON_SUBTREE.to_string(),
            HTMLContent::Plain("true".to_string()),
        );
    }

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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::super::{parse_markdown_sections_from_source, parse_markdown_source};
    use super::*;
    use crate::compiler::anonymous_slug::ANON_SUBTREE_SLUG_PREFIX;
    use crate::{
        compiler::HTMLContent,
        entry::{MetaData, KEY_INTERNAL_ANON_SUBTREE, KEY_SOURCE_POS, KEY_SOURCE_SLUG},
        slug::Slug,
    };

    #[test]
    fn test_extract_shared_reference_definitions_skips_fenced_code() {
        let source = r#"
[outside]: https://outside.example
```md
[inside-fence]: https://inside.example
```
[outside]: https://outside-duplicate.example
"#;

        let extracted = extract_subtrees_root(source, Slug::new("index")).unwrap();
        let shared = extract_shared_reference_definitions(&extracted.root_source);

        assert!(shared.contains("[outside]: https://outside.example"));
        assert!(!shared.contains("inside-fence"));
        assert!(!shared.contains("outside-duplicate"));
    }

    #[test]
    fn test_subtree_can_use_shared_reference_definitions_from_root() {
        let source = r#"
<remark slug="child">

[Subtree][shared-ref]

</remark>

[shared-ref]: https://example.com/shared
"#;

        let extracted = extract_subtrees_root(source, Slug::new("book/index")).unwrap();
        let shared = extract_shared_reference_definitions(&extracted.root_source);
        let subtree_source = compose_subtree_source(&extracted.subtrees[0].body, &shared);

        let child = parse_markdown_source(&subtree_source, extracted.subtrees[0].slug).unwrap();
        let html = child.content.as_str().unwrap_or_default();
        assert!(html.contains(r#"href="https://example.com/shared""#));
    }

    #[test]
    fn test_subtree_reference_definition_overrides_shared_root_definition() {
        let source = r#"
<remark slug="child">

[Subtree][same]

[same]: https://example.com/inner

</remark>

[same]: https://example.com/outer
"#;

        let extracted = extract_subtrees_root(source, Slug::new("book/index")).unwrap();
        let shared = extract_shared_reference_definitions(&extracted.root_source);
        let subtree_source = compose_subtree_source(&extracted.subtrees[0].body, &shared);

        let child = parse_markdown_source(&subtree_source, extracted.subtrees[0].slug).unwrap();
        let html = child.content.as_str().unwrap_or_default();
        assert!(html.contains(r#"href="https://example.com/inner""#));
        assert!(!html.contains(r#"href="https://example.com/outer""#));
    }

    #[test]
    fn test_extract_subtrees_rewrites_root_to_embed_placeholders() {
        let source = "before\n<remark slug=\"child\" title=\"Child\">\nhello\n</remark>\nafter";
        let extracted = extract_subtrees_root(source, Slug::new("doc/index")).unwrap();

        assert!(extracted
            .root_source
            .contains("[](/__kodama_subtree_internal__/0#:embed)"));
        assert_eq!(extracted.subtrees.len(), 1);
        assert_eq!(extracted.subtrees[0].slug, Slug::new("doc/child"));
        assert_eq!(extracted.subtrees[0].title.as_deref(), Some("Child"));
        assert_eq!(extracted.subtrees[0].tag, "remark");
        assert_eq!(extracted.subtrees[0].source_slug, Slug::new("doc/index"));
        assert_eq!(extracted.subtrees[0].source_pos, "2:1");
    }

    #[test]
    fn test_extract_subtrees_extracts_anonymous_tags_without_slug_attribute() {
        let source = "<remark>plain</remark>";
        let extracted = extract_subtrees_root(source, Slug::new("index")).unwrap();
        assert_eq!(extracted.subtrees.len(), 1);
        assert!(extracted
            .root_source
            .contains("[](/__kodama_subtree_internal__/0#:embed)"));
        assert!(extracted.subtrees[0].anonymous);
        assert!(extracted.subtrees[0]
            .slug
            .as_str()
            .contains(ANON_SUBTREE_SLUG_PREFIX));
    }
    #[test]
    fn test_extract_subtrees_nested_anonymous_slug_uses_source_prefix() {
        let source = "<remark>plain</remark>";
        let extracted = extract_subtrees_nested(
            source,
            Slug::new("daily/surf"),
            Slug::new("daily-surf/windows-skill"),
        )
        .unwrap();

        assert_eq!(extracted.subtrees.len(), 1);
        assert_eq!(
            extracted.subtrees[0].slug,
            Slug::new("daily-surf/windows-skill/:0")
        );
    }
    #[test]
    fn test_extract_subtrees_treats_anonymous_wrapper_as_single_extracted_subtree() {
        let source = r#"
<remark>

<proof slug="child">inner</proof>

</remark>"#;
        let extracted = extract_subtrees_root(source, Slug::new("index")).unwrap();
        assert_eq!(extracted.subtrees.len(), 1);
        assert!(extracted.subtrees[0].anonymous);
        assert!(extracted
            .subtrees
            .first()
            .map(|spec| spec.body.contains("<proof slug=\"child\">inner</proof>"))
            .unwrap_or(false));
    }

    #[test]
    fn test_parse_markdown_sections_anonymous_subtree_has_internal_section_structure() {
        let source = r#"
<remark>
anonymous body
</remark>
"#;
        let extracted = extract_subtrees_root(source, Slug::new("index")).unwrap();
        let mut root = parse_markdown_source(&extracted.root_source, Slug::new("index")).unwrap();
        patch_root_subtree_embeds(&mut root, &extracted.subtrees).unwrap();
        let mut anonymous =
            parse_markdown_source(&extracted.subtrees[0].body, extracted.subtrees[0].slug).unwrap();
        apply_subtree_defaults(&mut anonymous, &extracted.subtrees[0]);

        assert_eq!(extracted.subtrees.len(), 1);
        assert!(extracted.subtrees[0].anonymous);
        let HTMLContent::Lazy(contents) = &root.content else {
            panic!("expected lazy root content");
        };
        let embed_urls: Vec<_> = contents
            .iter()
            .filter_map(|content| match content {
                LazyContent::Embed(embed) => Some(embed.url.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(embed_urls.len(), 1);
        assert!(embed_urls[0].contains(ANON_SUBTREE_SLUG_PREFIX));
        assert!(anonymous
            .metadata
            .get(KEY_INTERNAL_ANON_SUBTREE)
            .and_then(HTMLContent::as_string)
            .is_some_and(|v| v == "true"));
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
        let extracted = extract_subtrees_root(source, Slug::new("book/index")).unwrap();
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
        assert_eq!(
            child
                .metadata
                .get(KEY_SOURCE_SLUG)
                .and_then(HTMLContent::as_string)
                .map(String::as_str),
            Some("book/index")
        );
        assert_eq!(
            child
                .metadata
                .get(KEY_SOURCE_POS)
                .and_then(HTMLContent::as_string)
                .map(String::as_str),
            Some("5:1")
        );
    }

    #[test]
    fn test_extract_subtrees_rejects_relative_prefix_slug() {
        let source = r#"<remark slug="./child">x</remark>"#;
        let err = extract_subtrees_root(source, Slug::new("doc/index")).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("invalid subtree tag"));
        assert!(msg.contains(r#"slug="./child""#));
    }

    #[test]
    fn test_extract_subtrees_rejects_absolute_or_nested_slug() {
        let absolute = r#"<remark slug="/child">x</remark>"#;
        let nested = r#"<remark slug="a/b">x</remark>"#;

        let absolute_err = extract_subtrees_root(absolute, Slug::new("doc/index")).unwrap_err();
        let nested_err = extract_subtrees_root(nested, Slug::new("doc/index")).unwrap_err();
        assert!(absolute_err.to_string().contains(r#"slug="/child""#));
        assert!(nested_err.to_string().contains(r#"slug="a/b""#));
    }

    #[test]
    fn test_parse_markdown_sections_supports_nested_named_subtrees() {
        let source = r#"
<remark slug="parent" title="Parent">
outer
<lemma slug="child" title="Child">inner</lemma>
</remark>
"#;

        let sections = parse_markdown_sections_from_source(source, Slug::new("index")).unwrap();
        assert!(sections.iter().any(|(slug, _)| *slug == Slug::new("index")));
        assert!(sections
            .iter()
            .any(|(slug, _)| *slug == Slug::new("parent")));
        assert!(sections.iter().any(|(slug, _)| *slug == Slug::new("child")));

        let parent = sections
            .iter()
            .find_map(|(slug, section)| (*slug == Slug::new("parent")).then_some(section))
            .expect("parent section should exist");
        let HTMLContent::Lazy(contents) = &parent.content else {
            panic!("expected lazy parent content with subtree embed");
        };
        let embeds: Vec<_> = contents
            .iter()
            .filter_map(|content| match content {
                LazyContent::Embed(embed) => Some(embed.url.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(embeds, vec!["/child".to_string()]);
    }

    #[test]
    fn test_parse_markdown_sections_nested_named_subtree_under_anonymous_wrapper_uses_visible_prefix(
    ) {
        let source = r#"
<remark>
<lemma slug="child" title="Child">inner</lemma>
</remark>
"#;

        let sections =
            parse_markdown_sections_from_source(source, Slug::new("book/index")).unwrap();
        assert!(sections
            .iter()
            .any(|(slug, _)| *slug == Slug::new("book/child")));

        let anonymous = sections
            .iter()
            .find_map(|(_, section)| {
                section
                    .metadata
                    .get(KEY_INTERNAL_ANON_SUBTREE)
                    .and_then(HTMLContent::as_string)
                    .is_some_and(|value| value == "true")
                    .then_some(section)
            })
            .expect("anonymous wrapper section should exist");
        let HTMLContent::Lazy(contents) = &anonymous.content else {
            panic!("expected lazy anonymous content with nested subtree embed");
        };
        let embeds: Vec<_> = contents
            .iter()
            .filter_map(|content| match content {
                LazyContent::Embed(embed) => Some(embed.url.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(embeds, vec!["/book/child".to_string()]);
    }

    #[test]
    fn test_parse_markdown_sections_nested_anonymous_subtrees_have_unique_internal_slugs() {
        let source = r#"
<remark>
<proof>a</proof>
</remark>
<lemma>
<proof>b</proof>
</lemma>
"#;

        let sections = parse_markdown_sections_from_source(source, Slug::new("GALA")).unwrap();
        let anonymous_slugs: Vec<Slug> = sections
            .iter()
            .filter_map(|(slug, section)| {
                section
                    .metadata
                    .get(KEY_INTERNAL_ANON_SUBTREE)
                    .and_then(HTMLContent::as_string)
                    .is_some_and(|value| value == "true")
                    .then_some(*slug)
            })
            .collect();

        assert_eq!(anonymous_slugs.len(), 4);
        let unique: HashSet<Slug> = anonymous_slugs.iter().copied().collect();
        assert_eq!(unique.len(), 4);
    }
}
