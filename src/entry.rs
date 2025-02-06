use crate::{config, html, html_flake::html_entry_header};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryMetaData(pub HashMap<String, String>);

const PRESET_METADATA: [&'static str; 3] = ["taxon", "title", "slug"];

impl EntryMetaData {
    pub fn to_header(&self, adhoc_title: Option<&str>, adhoc_taxon: Option<&str>) -> String {
        let entry_taxon = self.taxon_text().map_or("", |s| s);
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

    pub fn id(&self) -> String {
        crate::slug::to_hash_id(self.get("slug").unwrap())
    }

    pub fn taxon_text(&self) -> Option<&String> {
        return self.0.get("taxon");
    }

    pub fn title(&self) -> Option<&String> {
        return self.0.get("title");
    }

    pub fn slug(&self) -> Option<&String> {
        return self.0.get("slug");
    }

    pub fn update(&mut self, key: String, value: String) {
        let _ = self.0.insert(key, value);
    }
}
