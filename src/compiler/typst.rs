use super::html_parser::{HTMLParser, HTMLTagKind};
use super::section::{EmbedContent, LocalLink, SectionOption};
use super::section::{HTMLContent, HTMLContentBuilder, LazyContent};
use super::{CompileError, ShallowSection};
use crate::entry::HTMLMetaData;
use crate::slug::to_slug;
use crate::typst_cli;
use std::borrow::Cow;
use std::collections::HashMap;
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
    relative_path: &str,
    metadata: &mut HashMap<String, HTMLContent>,
) -> Result<HTMLContent, CompileError> {
    let mut builder = HTMLContentBuilder::new();
    let mut cursor: usize = 0;

    for span in HTMLParser::new(&html_str) {
        builder.push_str(&html_str[cursor..span.start]);
        cursor = span.end;

        let attr = |attr_name: &str| {
            span.attrs.get(attr_name).ok_or(CompileError::Syntax(
                Some(concat!(file!(), '#', line!())),
                Box::new(format!("No attribute '{}' in a kodama tag", attr_name)),
                relative_path.to_string(),
            ))
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
                let content = if let Some(value) = span.attrs.get("value") {
                    HTMLContent::Plain(value.to_string())
                } else {
                    parse_typst_html(span.body, relative_path, &mut HashMap::new())?
                };
                metadata.insert(attr("key")?.to_string(), content);
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
                let slug = to_slug(attr("slug")?);
                let text = value();
                builder.push(LazyContent::Local(LocalLink { slug, text }))
            }
        }
    }

    builder.push_str(&html_str[cursor..]);

    Ok(builder.build())
}

pub fn parse_typst(slug: &str, root_dir: &str) -> Result<ShallowSection, CompileError> {
    let relative_path = format!("{}.typst", slug);
    let html_str = typst_cli::file_to_html(&relative_path, root_dir).map_err(|e| {
        CompileError::IO(
            Some(concat!(file!(), '#', line!())),
            e,
            relative_path.to_string(),
        )
    })?;

    let mut metadata: HashMap<String, HTMLContent> = HashMap::new();
    metadata.insert("slug".to_string(), HTMLContent::Plain(slug.to_string()));

    let content = parse_typst_html(&html_str, &relative_path, &mut metadata)?;

    Ok(ShallowSection {
        metadata: HTMLMetaData(metadata),
        content,
    })
}
