use crate::{config, html, html_flake::html_entry_header};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryMetaData(pub HashMap<String, String>);

pub const KEY_TITLE: &'static str = "title";
pub const KEY_SLUG: &'static str = "slug";
pub const KEY_TAXON: &'static str = "taxon";

/// Control the "Previous Level" information in the current page navigation.
pub const KEY_PARENT: &'static str = "parent";
pub const KEY_PAGE_TITLE: &'static str = "page-title";

/// `backlinks: bool`: 
/// Controls whether the current page displays backlinks.
pub const KEY_BACKLINKS: &'static str = "backlinks";

/// `collect: bool`: 
/// Controls whether the current page is a collection page. 
/// A collection page displays metadata of child entries. 
pub const KEY_COLLECT: &'static str = "collect";

/// `asref: bool`: 
/// Controls whether the current page process as reference. 
pub const KEY_ASREF: &'static str = "asref";

const PRESET_METADATA: [&'static str; 8] = [
    KEY_TITLE,
    KEY_SLUG,
    KEY_TAXON,
    KEY_PARENT,
    KEY_PAGE_TITLE,
    KEY_BACKLINKS,
    KEY_COLLECT, 
    KEY_ASREF, 
];

impl EntryMetaData {
    pub fn to_header(&self, adhoc_title: Option<&str>, adhoc_taxon: Option<&str>) -> String {
        let entry_taxon = self.taxon().map_or("", |s| s);
        let taxon = adhoc_taxon.unwrap_or(entry_taxon);
        let entry_title = self.0.get("title").map(|s| s.as_str()).unwrap_or("");
        let title = adhoc_title.unwrap_or(entry_title);

        let slug = self.get("slug").unwrap();
        let slug_text = EntryMetaData::to_slug_text(&slug);
        let slug_url = config::full_html_url(&slug);
        let span_class: Vec<String> = vec!["taxon".to_string()];

        html!(header =>
          (html!(h1 =>
            (html!(span class = {span_class.join(" ")} => {taxon}))
            {title} {" "}
            (html!(a class = "slug", href = {slug_url} => "["{&slug_text}"]"))))
          (html!(html_entry_header(self.etc()))))
    }

    /// hidden suffix `/index` in slug text.
    pub fn to_slug_text(slug: &String) -> String {
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

    pub fn is_custom_metadata(s: &str) -> bool {
        !PRESET_METADATA.contains(&s)
    }

    pub fn enabled_markdown_key(s: &str) -> bool {
        EntryMetaData::is_custom_metadata(s)
    }

    pub fn enabled_markdown_keys(&self) -> Vec<String> {
        self.0
            .keys()
            .filter(|s| EntryMetaData::enabled_markdown_key(s))
            .map(|s| s.to_string())
            .collect()
    }

    /// Return all custom metadata keys without [`PRESET_METADATA`].
    pub fn etc_keys(&self) -> Vec<String> {
        self.0
            .keys()
            .filter(|s| EntryMetaData::is_custom_metadata(s))
            .map(|s| s.to_string())
            .collect()
    }

    /// Return all custom metadata values without [`PRESET_METADATA`].
    pub fn etc(&self) -> Vec<String> {
        let mut etc_keys = self.etc_keys();
        etc_keys.sort();
        etc_keys
            .into_iter()
            .map(|s| self.get(&s).unwrap().to_string())
            .collect()
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        return self.0.get(key);
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.0.get(key).map(|s| s == "true")
    }

    pub fn id(&self) -> String {
        crate::slug::to_hash_id(self.get(KEY_SLUG).unwrap())
    }

    /// Return taxon text
    pub fn taxon(&self) -> Option<&String> {
        return self.0.get(KEY_TAXON);
    }

    pub fn title(&self) -> Option<&String> {
        return self.0.get(KEY_TITLE);
    }

    #[allow(dead_code)]
    pub fn page_title(&self) -> Option<&String> {
        return self.0.get(KEY_PAGE_TITLE);
    }

    pub fn slug(&self) -> Option<&String> {
        return self.0.get(KEY_SLUG);
    }

    pub fn is_enable_backliks(&self) -> bool {
        return self.get_bool(&KEY_BACKLINKS).unwrap_or(true);
    }

    pub fn is_collect(&self) -> bool {
        return self.get_bool(&KEY_COLLECT).unwrap_or(false);
    }

    pub fn is_asref(&self) -> bool {
        return self.get_bool(&KEY_ASREF).unwrap_or(false);
    }

    pub fn update(&mut self, key: String, value: String) {
        let _ = self.0.insert(key, value);
    }
}
