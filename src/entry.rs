// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use crate::{
    compiler::{section::HTMLContent, taxon::Taxon},
    config::{self, FooterMode},
    html_flake,
    ordered_map::OrderedMap,
    slug::Slug,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTMLMetaData(pub OrderedMap<String, HTMLContent>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryMetaData(pub OrderedMap<String, String>);

pub const KEY_TITLE: &str = "title";
pub const KEY_SLUG: &str = "slug";
pub const KEY_TAXON: &str = "taxon";
pub const KEY_DATA_TAXON: &str = "data-taxon";

/// Control the "Previous Level" information in the current page navigation.
pub const KEY_PARENT: &str = "parent";

/// Control the page title text of the current page.
pub const KEY_PAGE_TITLE: &str = "page-title";

/// `backlinks: bool`:
/// Controls whether the current page displays backlinks.
pub const KEY_BACKLINKS: &str = "backlinks";

/// `collect: bool`:
/// Controls whether the current page is a collection page.
/// A collection page displays metadata of child entries.
pub const KEY_COLLECT: &str = "collect";

/// `asref: bool`:
/// Controls whether the current page process as reference.
pub const KEY_ASREF: &str = "asref";

/// `footer-mode: embed | link`
pub const KEY_FOOTER_MODE: &str = "footer-mode";

const PRESET_METADATA: [&str; 10] = [
    KEY_TITLE,
    KEY_SLUG,
    KEY_TAXON,
    KEY_DATA_TAXON,
    KEY_PARENT,
    KEY_PAGE_TITLE,
    KEY_BACKLINKS,
    KEY_COLLECT,
    KEY_ASREF,
    KEY_FOOTER_MODE,
];

pub trait MetaData<V>
where
    V: Clone,
{
    fn get(&self, key: &str) -> Option<&V>;
    fn get_str(&self, key: &str) -> Option<&String>;
    fn keys(&self) -> impl Iterator<Item = &String>;

    fn is_custom_metadata(s: &str) -> bool {
        !PRESET_METADATA.contains(&s)
    }

    /// Return all custom metadata keys without [`PRESET_METADATA`].
    fn etc_keys(&self) -> Vec<String> {
        self.keys()
            .filter(|s| EntryMetaData::is_custom_metadata(s))
            .map(|s| s.to_string())
            .collect()
    }

    /// Return all custom metadata values without [`PRESET_METADATA`].
    fn etc(&self) -> Vec<V> {
        self.etc_keys()
            .into_iter()
            .map(|s| self.get(&s).unwrap().clone())
            .collect()
    }

    fn get_bool(&self, key: &str) -> Option<bool> {
        self.get_str(key).map(|s| s == "true")
    }

    fn id(&self) -> String {
        crate::slug::to_hash_id(self.get_str(KEY_SLUG).unwrap())
    }

    /// Return taxon text
    fn taxon(&self) -> Option<&V> {
        self.get(KEY_TAXON)
    }

    fn data_taxon(&self) -> Option<&String> {
        self.get_str(KEY_DATA_TAXON)
    }

    fn title(&self) -> Option<&V> {
        self.get(KEY_TITLE)
    }

    fn page_title(&self) -> Option<&String> {
        self.get_str(KEY_PAGE_TITLE)
    }

    fn slug(&self) -> Option<Slug> {
        self.get_str(KEY_SLUG).map(Slug::new)
    }

    fn is_enable_backlinks(&self) -> bool {
        self.get_bool(KEY_BACKLINKS).unwrap_or(true)
    }

    fn is_collect(&self) -> bool {
        self.get_bool(KEY_COLLECT).unwrap_or(false)
    }

    fn is_asref(&self) -> bool {
        self.get_bool(KEY_ASREF).unwrap_or(false)
    }
}

impl MetaData<HTMLContent> for HTMLMetaData {
    fn get(&self, key: &str) -> Option<&HTMLContent> {
        self.0.get(key)
    }

    fn get_str(&self, key: &str) -> Option<&String> {
        self.0.get(key).and_then(HTMLContent::as_string)
    }

    fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.keys()
    }
}

impl MetaData<String> for EntryMetaData {
    fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    fn get_str(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.keys()
    }
}

impl HTMLMetaData {
    pub fn compute_textual_attrs(&mut self) {
        if self.page_title().is_none() {
            if let Some(title) = self.title() {
                self.0.insert(
                    KEY_PAGE_TITLE.to_string(),
                    HTMLContent::Plain(title.remove_all_tags()),
                );
            }
        }

        if self.data_taxon().is_none() {
            if let Some(taxon) = self.taxon() {
                self.0.insert(
                    KEY_DATA_TAXON.to_string(),
                    HTMLContent::Plain(Taxon::to_data_taxon(&taxon.remove_all_tags()).to_string()),
                );
            }
        }
    }
}

impl EntryMetaData {
    pub fn to_header(&self, adhoc_title: Option<&str>, adhoc_taxon: Option<&str>) -> String {
        let entry_taxon = self.taxon().map_or("", |s| s);
        let taxon = adhoc_taxon.unwrap_or(entry_taxon);
        let entry_title = self.0.get("title").map(|s| s.as_str()).unwrap_or("");
        let title = adhoc_title.unwrap_or(entry_title);
        let slug = Slug::new(self.get("slug").unwrap());
        let span_class: Vec<String> = vec!["taxon".to_string()];

        html_flake::html_header(title, taxon, &slug, span_class.join(" "), self.etc())
    }

    /// hidden suffix `/index` in slug text.
    pub fn to_slug_text(slug: &str) -> String {
        let mut slug_text = match slug.ends_with("/index") {
            true => &slug[..slug.len() - "/index".len()],
            false => slug,
        };
        if config::is_short_slug() {
            let pos = slug_text.rfind("/").map_or(0, |n| n + 1);
            slug_text = &slug_text[pos..];
        }
        slug_text.to_string()
    }

    pub fn update(&mut self, key: String, value: String) {
        let _ = self.0.insert(key, value);
    }

    pub fn footer_mode(&self) -> Option<FooterMode> {
        self.get_str(KEY_FOOTER_MODE).map(|s| {
            FooterMode::from_str(s).expect("footer-mode must be either `embed` or `link`.")
        })
    }
}
