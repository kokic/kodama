use std::borrow::Cow;
use std::cell::RefCell;
use std::str;
use std::{collections::HashMap, vec};

use quick_xml::events::{BytesStart, Event};
use quick_xml::reader::Reader;

use crate::compiler::section::{EmbedContent, LocalLink, SectionOption};
use crate::{entry::EntryMetaData, typst_cli::source_to_html_inplace};

use super::{section::LazyContent, CompileError, HTMLContent, ShallowSection};

fn is_positive(s: Cow<'_, str>) -> bool {
    !matches!(s.as_ref(), "false" | "0" | "none")
}

pub fn parse_typst(slug: &str, root_dir: &str) -> Result<ShallowSection, CompileError> {
    let relative_path = format!("{}.typst", slug);
    let html_str = source_to_html_inplace(&relative_path, root_dir).map_err(|e| {
        CompileError::IO(
            Some(concat!(file!(), '#', line!())),
            e,
            relative_path.to_string(),
        )
    })?;

    let mut metadata = HashMap::new();
    metadata.insert("slug".to_string(), slug.to_string());
    let mut contents = vec![];
    let content = RefCell::new(String::new());

    let mut tag_kodama = |e: BytesStart| {
        let mattr = |attr_name: &str| {
            e.try_get_attribute(attr_name)
                .map_err(|e| {
                    CompileError::Syntax(
                        Some(concat!(file!(), '#', line!())),
                        Box::new(e),
                        relative_path.to_string(),
                    )
                })
                .map(|o| o.map(|a| a.unescape_value().unwrap()))
        };
        let attr = |attr_name: &str| {
            mattr(attr_name)?.ok_or(CompileError::Syntax(
                Some(concat!(file!(), '#', line!())),
                Box::new(format!(
                    "No attribute named {} in tag {}",
                    attr_name,
                    &String::from_utf8_lossy(e.name().0)
                )),
                relative_path.to_string(),
            ))
        };
        match attr("type")?.as_ref() {
            "meta" => {
                metadata.insert(attr("key")?.to_string(), attr("value")?.to_string());
            }
            "embed" => {
                if !content.borrow().is_empty() {
                    contents.push(LazyContent::Plain(content.take()));
                }

                let def = SectionOption::default();
                let url = attr("url")?.to_string();
                let title = mattr("title")?.map(|s| s.to_string());
                let numbering = mattr("numbering")?.map_or(def.numbering, is_positive);
                let details_open = mattr("open")?.map_or(def.details_open, is_positive);
                let catalog = mattr("catalog")?.map_or(def.catalog, is_positive);
                contents.push(LazyContent::Embed(EmbedContent {
                    url,
                    title,
                    option: SectionOption::new(numbering, details_open, catalog),
                }))
            }
            "local" => {
                if !content.borrow().is_empty() {
                    contents.push(LazyContent::Plain(content.take()));
                }

                let slug = attr("slug")?.to_string();
                let text = mattr("text")?.map(|s| s.to_string());
                contents.push(LazyContent::Local(LocalLink { slug, text }))
            }
            tag => {
                return Err(CompileError::Syntax(
                    Some(concat!(file!(), '#', line!())),
                    Box::new(format!("Unknown kodama element type {}", tag)),
                    relative_path.to_string(),
                ))
            }
        }
        Ok(())
    };

    let mut reader = Reader::from_str(&html_str);
    reader.config_mut().trim_text(true);
    loop {
        match reader.read_event() {
            Err(e) => {
                return Err(CompileError::Syntax(
                    Some(concat!(file!(), '#', line!())),
                    Box::new(format!(
                        "Error at position {}: {:?}",
                        reader.error_position(),
                        e
                    )),
                    relative_path.to_string(),
                ))
            }
            Ok(Event::Eof) => break,

            Ok(Event::Start(e)) => match e.name().as_ref() {
                b"kodama" => tag_kodama(e)?,
                b"html" | b"body" => (),
                _ => content
                    .borrow_mut()
                    .push_str(&format!("<{}>", String::from_utf8_lossy(&e.to_vec()))),
            },
            Ok(Event::Empty(e)) => match e.name().as_ref() {
                b"kodama" => tag_kodama(e)?,
                b"html" | b"body" => (),
                _ => content
                    .borrow_mut()
                    .push_str(&format!("<{}/>", String::from_utf8_lossy(&e.to_vec()))),
            },
            Ok(Event::End(e)) => match e.name().as_ref() {
                b"kodama" => (),
                b"html" | b"body" => (),
                _ => content
                    .borrow_mut()
                    .push_str(&format!("</{}>", String::from_utf8_lossy(&e.to_vec()))),
            },
            Ok(Event::Comment(e)) => content
                .borrow_mut()
                .push_str(&format!("<!--{}-->", String::from_utf8_lossy(&e.to_vec()))),
            Ok(Event::Text(e)) => content
                .borrow_mut()
                .push_str(&String::from_utf8_lossy(&e.to_vec())),

            _ => (),
        }
    }

    if !content.borrow().is_empty() {
        contents.push(LazyContent::Plain(content.take()));
    }

    if contents.len() == 1 {
        if let LazyContent::Plain(html) = &contents[0] {
            return Ok(ShallowSection {
                metadata: EntryMetaData(metadata),
                content: HTMLContent::Plain(html.to_string()),
            });
        }
    }

    Ok(ShallowSection {
        metadata: EntryMetaData(metadata),
        content: HTMLContent::Lazy(contents),
    })
}
