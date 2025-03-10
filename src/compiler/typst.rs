use super::section::{HTMLContent, HTMLContentBuilder, LazyContent};
use super::{CompileError, ShallowSection};
use crate::compiler::section::{EmbedContent, LocalLink, SectionOption};
use crate::entry::HTMLMetaData;
use crate::slug::to_slug;
use crate::typst_cli;
use fancy_regex::Regex;
use std::collections::HashMap;
use std::str;

fn process_bool(m: Option<&String>, def: bool) -> bool {
    match m.map(String::as_str) {
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

    let pre_kodama = |tags: &str, alt: u8| {
        format!(
            r#"<kodama(?<tag{}>{})(?<attrs{}>(\s+([a-zA-Z]+)="([^"\\]|\\[\s\S])*")*)>(?<inner{}>[\s\S]*?)</kodama(?P=tag{})>"#,
            alt, tags, alt, alt, alt
        )
    };
    let re_kodama = Regex::new(&format!(
        "<span>{}</span>|{}",
        pre_kodama("local", 0),
        pre_kodama("meta|embed|local", 1)
    ))
    .unwrap();
    // println!("{}", re_kodama);
    let re_attrs = Regex::new(r#"(?<key>[a-zA-Z]+)="(?<value>([^"\\]|\\[\s\S])*)""#).unwrap();

    for capture in re_kodama.captures_iter(&html_str).map(Result::unwrap) {
        let all = capture.get(0).unwrap();
        let get_capture = |name: &str| {
            capture
                .name(&format!("{}0", name))
                .or(capture.name(&format!("{}1", name)))
        };

        builder.push_str(&html_str[cursor..all.start()]);
        cursor = all.end();

        let attrs_str = get_capture("attrs").unwrap().as_str();
        let attrs: HashMap<&str, String> = re_attrs
            .captures_iter(attrs_str)
            .map(Result::unwrap)
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

        let value = || {
            let value = attrs.get("value").map_or_else(
                || get_capture("inner").unwrap().as_str().trim().to_string(),
                |s| s.to_string(),
            );
            if value.is_empty() {
                None
            } else {
                Some(value)
            }
        };
        match get_capture("tag").unwrap().as_str() {
            "meta" => {
                let content = if let Some(value) = attrs.get("value") {
                    HTMLContent::Plain(value.to_string())
                } else {
                    parse_typst_html(
                        get_capture("inner").unwrap().as_str().trim(),
                        relative_path,
                        &mut HashMap::new(),
                    )?
                };
                metadata.insert(attr("key")?.to_string(), content);
            }
            "embed" => {
                let def = SectionOption::default();

                let url = attr("url")?.to_string();
                let title = value();
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
                let slug = to_slug(attr("slug")?);
                let text = value();
                builder.push(LazyContent::Local(LocalLink { slug, text }))
            }
            tag => {
                return Err(CompileError::Syntax(
                    Some(concat!(file!(), '#', line!())),
                    Box::new(format!("Unknown kodama element type {}", tag)),
                    relative_path.to_string(),
                ))
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
