use serde::{Deserialize, Serialize};

use crate::entry::EntryMetaData;

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

#[derive(Debug, Serialize, Deserialize)]
pub struct EmbedContent {
    pub url: String,
    pub title: Option<String>,
    pub option: SectionOption,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LocalLink {
    pub slug: String,
    pub text: Option<String>,
}

/// Plain HTMLs & lazy embedding HTMLs, This means that
/// the embedded structure within are not expanded.
#[derive(Debug, Serialize, Deserialize)]
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
#[derive(Debug, Serialize, Deserialize)]
pub enum HTMLContent {
    Plain(String),
    Lazy(LazyContents),
}

///
#[derive(Debug, Serialize, Deserialize)]
pub struct ShallowSection {
    pub metadata: EntryMetaData,
    pub content: HTMLContent,
}

impl ShallowSection {
    pub fn slug(&self) -> String {
        self.metadata.slug().unwrap().to_string()
    }

    pub fn is_compiled(&self) -> bool {
        matches!(&self.content, HTMLContent::Plain(_)) && self.metadata.etc_keys().len() == 0
    }

    /// Convert a [`ShallowSection`] containing only [`HTMLContent::Plain`] to a [`Section`].
    /// For other cases, this method will directly call [`unreachable!`] and lead to a [`panic`].
    pub fn to_section(self) -> Section {
        match &self.content {
            HTMLContent::Lazy(_) => unreachable!(),
            HTMLContent::Plain(html) => {
                let content = SectionContent::Plain(html.to_string());
                Section::new(self.metadata, vec![content])
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Section {
    pub metadata: EntryMetaData,
    pub children: SectionContents,
    pub option: SectionOption,
}

pub type SectionContents = Vec<SectionContent>;

#[derive(Debug, Clone)]
pub enum SectionContent {
    Plain(String),
    Embed(Section),
}

impl Section {
    pub fn new(metadata: EntryMetaData, children: SectionContents) -> Section {
        Section {
            metadata,
            children,
            option: SectionOption::new(false, true, true),
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

// #[derive(Debug)]
// pub struct CompiledEntry {
// pub catalog: Catalog,
// pub metadata: EntryMetaData,
// pub section: Section,
// }

// impl CompiledEntry {
//     pub fn get(&self, key: &str) -> Option<&String> {
//         return self.metadata.get(key);
//     }

//     pub fn update(&mut self, key: String, value: String) {
//         let _ = self.metadata.0.insert(key, value);
//     }
// }
