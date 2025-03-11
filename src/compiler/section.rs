use std::{collections::HashSet, mem};

use lazy_static::lazy_static;
use regex_lite::Regex;
use serde::{Deserialize, Serialize};

use crate::entry::{EntryMetaData, HTMLMetaData, MetaData};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionOption {
    pub numbering: bool, // default: false

    /// Display children catalog
    pub details_open: bool, // default: true

    /// Display in catalog
    pub catalog: bool, // default: true
}

impl Default for SectionOption {
    fn default() -> Self {
        SectionOption::new(false, true, true)
    }
}

impl SectionOption {
    pub fn new(numbering: bool, details_open: bool, catalog: bool) -> SectionOption {
        SectionOption {
            numbering,
            details_open,
            catalog,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedContent {
    pub url: String,
    pub title: Option<String>,
    pub option: SectionOption,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalLink {
    pub slug: String,
    pub text: Option<String>,
}

/// Plain HTMLs & lazy embedding HTMLs, This means that
/// the embedded structure within are not expanded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LazyContent {
    Plain(String),
    Embed(EmbedContent),
    Local(LocalLink),
}

pub type LazyContents = Vec<LazyContent>;

/// The purpose of this structure is to handle cases like [`LocalLink`],
/// where full information cannot be directly obtained during the parsing stage.
///
/// Additionally, it is designed with the consideration that
/// when all contents in `Vec<LazyContent>` are [`LazyContent::Plain`],
/// this structure will naturally be lifted to [`HTMLContent::Plain`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HTMLContent {
    Plain(String),
    Lazy(LazyContents),
}

impl HTMLContent {
    pub fn as_str(&self) -> Option<&str> {
        if let HTMLContent::Plain(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_string(&self) -> Option<&String> {
        if let HTMLContent::Plain(s) = self {
            Some(s)
        } else {
            None
        }
    }

    fn remove_tag(s: &str, regex: &Regex) -> String {
        let mut cursor = 0;
        let mut string = String::new();
        for capture in regex.captures_iter(s) {
            let all = capture.get(0).unwrap();
            string.push_str(&s[cursor..all.start()]);
            cursor = all.end();
        }
        string.push_str(&s[cursor..]);
        string
    }

    fn to_some_title(&self, regex: &Regex) -> String {
        match self {
            HTMLContent::Plain(s) => HTMLContent::remove_tag(s, regex),
            HTMLContent::Lazy(contents) => {
                let mut str = String::new();
                for content in contents {
                    match content {
                        LazyContent::Plain(s) => str.push_str(&HTMLContent::remove_tag(s, regex)),
                        LazyContent::Embed(embed) => str
                            .push_str(embed.title.as_ref().map(String::as_str).unwrap_or_default()),
                        LazyContent::Local(local) => str
                            .push_str(local.text.as_ref().map(String::as_str).unwrap_or_default()),
                    }
                }
                str
            }
        }
    }

    pub fn to_page_title(&self) -> String {
        lazy_static! {
            static ref re_tags: Regex = {
                let attrs = r#"(\s+[a-zA-Z-]+="([^"\\]|\\[\s\S])*")*"#;
                Regex::new(&format!(
                    r#"<[A-Za-z]+{}>|</[A-Za-z]+>|<[A-Za-z]+{}/>"#,
                    attrs, attrs
                ))
                .unwrap()
            };
        }
        self.to_some_title(&re_tags)
    }

    pub fn to_link_title(&self) -> String {
        lazy_static! {
            static ref re_tag_a: Regex = {
                let attrs = r#"(\s+[a-zA-Z-]+="([^"\\]|\\[\s\S])*")*"#;
                Regex::new(&format!(r#"<a{}>|</a>|<a{}/>"#, attrs, attrs)).unwrap()
            };
        }
        self.to_some_title(&re_tag_a)
    }
}

pub struct HTMLContentBuilder {
    contents: LazyContents,
    content: String,
}

impl HTMLContentBuilder {
    pub fn new() -> HTMLContentBuilder {
        HTMLContentBuilder {
            contents: vec![],
            content: String::new(),
        }
    }
    pub fn push_str(&mut self, s: &str) {
        if !s.is_empty() {
            self.content.push_str(&s);
        }
    }
    fn push_content(&mut self) {
        if !self.content.is_empty() {
            self.contents
                .push(LazyContent::Plain(mem::take(&mut self.content)));
        }
    }
    pub fn push(&mut self, c: LazyContent) {
        match c {
            LazyContent::Plain(s) => {
                self.push_str(&s);
            }
            _ => {
                self.push_content();
                self.contents.push(c);
            }
        }
    }
    pub fn build(mut self) -> HTMLContent {
        if self.contents.is_empty() {
            return HTMLContent::Plain(mem::take(&mut self.content));
        }
        self.push_content();
        HTMLContent::Lazy(self.contents)
    }
}

///
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShallowSection {
    pub metadata: HTMLMetaData,
    pub content: HTMLContent,
}

impl ShallowSection {
    pub fn slug(&self) -> String {
        self.metadata.slug().unwrap().to_string()
    }

    #[allow(dead_code)]
    pub fn is_compiled(&self) -> bool {
        matches!(&self.content, HTMLContent::Plain(_)) && self.metadata.etc_keys().len() == 0
    }
}

pub type SectionContents = Vec<SectionContent>;

#[derive(Debug, Clone)]
pub enum SectionContent {
    Plain(String),
    Embed(Section),
}

#[derive(Debug, Clone)]
pub struct Section {
    pub metadata: EntryMetaData,
    pub children: SectionContents,
    pub option: SectionOption,
    pub references: HashSet<String>,
}

impl Section {
    pub fn new(
        metadata: EntryMetaData,
        children: SectionContents,
        references: HashSet<String>,
    ) -> Section {
        Section {
            metadata,
            children,
            option: SectionOption::new(false, true, true),
            references,
        }
    }

    pub fn slug(&self) -> String {
        self.metadata.slug().unwrap().to_string()
    }

    pub fn spanned(&self) -> String {
        self.children
            .iter()
            .map(|content| match content {
                SectionContent::Plain(html) => html.to_string(),
                SectionContent::Embed(_) => unreachable!(),
            })
            .reduce(|s, t| s + &t)
            .unwrap_or_default()
    }
}
