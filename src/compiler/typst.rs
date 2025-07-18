// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Alias Qli (@AliasQli), Spore (@s-cerevisiae), Kokic (@kokic)

use eyre::{eyre, WrapErr};

use super::html_parser::{HTMLParser, HTMLTagKind};
use super::section::{EmbedContent, LocalLink, SectionOption};
use super::section::{HTMLContent, HTMLContentBuilder, LazyContent};
use super::ShallowSection;
use crate::entry::HTMLMetaData;
use crate::ordered_map::OrderedMap;
use crate::process::embed_markdown;
use crate::slug::Slug;
use crate::typst_cli;
use std::borrow::Cow;
use std::path::Path;
use std::str;

fn parse_bool(m: Option<&Cow<'_, str>>, def: bool) -> bool {
    match m.map(|s| s.as_ref()) {
        None | Some("auto") => def,
        Some("false") | Some("0") | Some("none") => false,
        _ => true,
    }
}

fn parse_typst_html(
    html_str: &str,
    metadata: &mut OrderedMap<String, HTMLContent>,
) -> eyre::Result<HTMLContent> {
    let mut builder = HTMLContentBuilder::new();
    let mut cursor: usize = 0;

    for span in HTMLParser::new(html_str) {
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
                    parse_typst_html(span.body, &mut OrderedMap::new())?
                };
                if key == "taxon" {
                    if let HTMLContent::Plain(v) = val {
                        val = HTMLContent::Plain(embed_markdown::display_taxon(&v));
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
                let url = attr("slug")?.to_string();
                let text = value();
                builder.push(LazyContent::Local(LocalLink { url, text }))
            }
        }
    }

    builder.push_str(&html_str[cursor..]);

    Ok(builder.build())
}

pub fn parse_typst<P: AsRef<Path>>(slug: Slug, root_dir: P) -> eyre::Result<ShallowSection> {
    let typst_root_dir = root_dir.as_ref().to_string_lossy();
    let relative_path = format!("{}.typst", slug);
    let html_str = typst_cli::file_to_html(&relative_path, typst_root_dir.as_ref())
        .wrap_err_with(|| eyre!("failed to compile typst file `{relative_path}` to html"))?;

    let mut metadata: OrderedMap<String, HTMLContent> = OrderedMap::new();
    metadata.insert("slug".to_string(), HTMLContent::Plain(slug.to_string()));

    let content = parse_typst_html(&html_str, &mut metadata)?;

    Ok(ShallowSection {
        metadata: HTMLMetaData(metadata),
        content,
    })
}
