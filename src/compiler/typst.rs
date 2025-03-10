use super::section::HTMLContentBuilder;
use super::{section::LazyContent, CompileError, ShallowSection};
use crate::compiler::section::{EmbedContent, LocalLink, SectionOption};
use crate::{entry::EntryMetaData, typst_cli};
use regex::Regex;
use std::collections::HashMap;
use std::str;

fn process_bool(m: Option<&String>, def: bool) -> bool {
    match m.map(String::as_str) {
        None | Some("auto") => def,
        Some("false") | Some("0") | Some("none") => false,
        _ => true,
    }
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
    let mut cursor: usize = 0;

    let mut metadata = HashMap::new();
    metadata.insert("slug".to_string(), slug.to_string());
    let mut builder = HTMLContentBuilder::new();

    let re_kodama = Regex::new(
        r#"<kodama(?<attrs>(\s+([a-zA-Z]+)="([^"\\]|\\[\s\S])*")*)>(?<inner>[\s\S]*?)</kodama>"#,
    )
    .unwrap();
    let re_attrs = Regex::new(r#"(?<key>[a-zA-Z]+)="(?<value>([^"\\]|\\[\s\S])*)""#).unwrap();

    for capture in re_kodama.captures_iter(&html_str) {
        let all = capture.get(0).unwrap();

        builder.push_str(&html_str[cursor..all.start()]);
        cursor = all.end();

        let attrs_str = capture.name("attrs").unwrap().as_str();
        let attrs: HashMap<&str, String> = re_attrs
            .captures_iter(attrs_str)
            .map(|c| {
                (
                    c.name("key").unwrap().as_str(),
                    String::from_utf8_lossy(
                        escape_bytes::unescape(c.name("value").unwrap().as_str().as_bytes())
                            .unwrap()
                            .as_slice(),
                    )
                    .into_owned(),
                )
            })
            .collect();

        let attr = |attr_name: &str| {
            attrs.get(attr_name).ok_or(CompileError::Syntax(
                Some(concat!(file!(), '#', line!())),
                Box::new(format!("No attribute '{}' in tag kodama", attr_name)),
                relative_path.to_string(),
            ))
        };

        let value = attrs.get("value").map_or_else(
            || capture.name("inner").unwrap().as_str().trim().to_string(),
            ToString::to_string,
        );
        fn str_opt(value: String) -> Option<String> {
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        }
        match attr("type")?.as_ref() {
            "meta" => {
                metadata.insert(attr("key")?.to_string(), value);
            }
            "embed" => {
                let def = SectionOption::default();

                let url = attr("url")?.to_string();
                let title = str_opt(value);
                let numbering = process_bool(attrs.get("numbering"), def.numbering);
                let details_open = process_bool(attrs.get("open"), def.details_open);
                let catalog = process_bool(attrs.get("catalog"), def.catalog);
                builder.push(LazyContent::Embed(EmbedContent {
                    url,
                    title,
                    option: SectionOption::new(numbering, details_open, catalog),
                }))
            }
            "local" => {
                let slug = attr("slug")?.to_string();
                let text = str_opt(value);
                builder.push(LazyContent::Local(LocalLink { slug, text }))
            }
            tag => {
                return Err(CompileError::Syntax(
                    Some(concat!(file!(), '#', line!())),
                    Box::new(format!("Unknown kodama element type {}", tag)),
                    relative_path,
                ))
            }
        }
    }

    builder.push_str(&html_str[cursor..]);

    Ok(ShallowSection {
        metadata: EntryMetaData(metadata),
        content: builder.build(),
    })
}
