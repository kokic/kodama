use crate::{config, html, html_flake::html_entry_header, recorder::Catalog};
use std::collections::HashMap;

// #[derive(serde::Serialize, serde::Deserialize)]
pub struct EntryMetaData(pub HashMap<String, String>);

const PRESET_METADATA: [&'static str; 3] = ["taxon", "title", "slug"];

impl EntryMetaData {
    pub fn to_header(&self) -> String {
        let taxon = self.taxon().map_or("", |s| s);
        let title = self.0.get("title").map(|s| s.as_str()).unwrap_or("");

        let slug = self.get("slug").unwrap();
        // hidden suffix `/index` in slug text
        let slug_text = EntryMetaData::to_slug_text(&slug);
        let slug_url = config::full_url(&format!("{}{}", &slug, config::page_suffix()));
        let span_class: Vec<String> = vec!["taxon".to_string()];

        html!(header =>
          (html!(h1 =>
            (html!(span class = {span_class.join(" ")} => {taxon}))
            {title}
            {" "}
            (html!(a class = "slug", href = {slug_url} => "["{&slug_text}"]"))))
          (html!(html_entry_header(self.etc()))))
    }

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

    /// return all custom metadata values
    pub fn etc(&self) -> Vec<String> {
        let mut etc_keys: Vec<&String> = self
            .0
            .keys()
            .filter(|s| EntryMetaData::is_custom_metadata(s))
            .collect();
        etc_keys.sort();
        etc_keys
            .into_iter()
            .map(|s| self.get(s).unwrap().to_string())
            .collect()
    }

    pub fn id(&self) -> String {
        crate::slug::to_id(self.get("slug").unwrap())
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        return self.0.get(key);
    }

    pub fn taxon(&self) -> Option<&String> {
        return self.0.get("taxon");
    }

    pub fn title(&self) -> Option<&String> {
        return self.0.get("title");
    }
}

// #[derive(serde::Serialize, serde::Deserialize)]
pub struct HtmlEntry {
    pub metadata: EntryMetaData,
    pub catalog: Catalog,
    pub content: String,
}

impl HtmlEntry {
    pub fn get(&self, key: &str) -> Option<&String> {
        return self.metadata.get(key);
    }

    pub fn update(&mut self, key: String, value: String) {
        let _ = self.metadata.0.insert(key, value);
    }
}
