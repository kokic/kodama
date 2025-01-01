use crate::{html, html_flake::html_entry_header, recorder::Catalog};
use std::collections::HashMap;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct EntryMetaData(pub HashMap<String, String>);

impl EntryMetaData {
    pub fn to_header(&self) -> String {
        let taxon = self.taxon().map_or("", |s| s);
        let title = self
            .0
            .get("title")
            .map(|s| s.as_str())
            .unwrap_or("[no_title]");

        let slug = self.get("slug").unwrap();
        let slug_url = format!("{}.html", &slug);

        let author = self
            .get("author")
            .map(|s| s.as_str())
            .unwrap_or("Anonymous");
        let start_date = self.get("date").or(self.get("start_date"));
        let end_date = self.get("end_date");
        let span_class: Vec<String> = vec!["taxon".to_string()];

        html!(header =>
          (html!(h1 =>
            (html!(span class = {span_class.join(" ")} => {taxon}))
            {title}
            {" "}
            (html!(a class = "slug", href = {slug_url} => "["{&slug}"]"))))
          (html!(html_entry_header(author, start_date, end_date, vec![]))))
    }

    pub fn slug_to_id(slug: &str) -> String {
        slug.replace("/", "-")
    }

    pub fn id(&self) -> String {
        Self::slug_to_id(self.get("slug").unwrap())
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

#[derive(serde::Serialize, serde::Deserialize)]
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
