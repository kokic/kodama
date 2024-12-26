use crate::{html, html_flake::html_entry_header};
use std::collections::HashMap;

pub struct EntryMetaData(pub HashMap<String, String>);

impl EntryMetaData {
    pub fn to_header(&self) -> String {
        let taxon = match self.0.get("taxon") {
            None => "".to_string(),
            Some(s) => {
                let (first, rest) = s.split_at(1);
                format!("{}. ", first.to_uppercase() + rest)
            }
        };
        let title = self
            .0
            .get("title")
            .map(|s| s.as_str())
            .unwrap_or("[No Title]");

        let slug = self.get("slug").unwrap();
        let slug_url = format!("{}.html", &slug);

        let author = self
            .get("author")
            .map(|s| s.as_str())
            .unwrap_or("Anonymous");
        let start_date = self.get("date").or(self.get("start_date"));
        let end_date = self.get("end_date");

        html!(header =>
          (html!(h1 =>
            (html!(span class = "taxon" => {taxon}))
            {title}
            {" "}
            (html!(a class = "slug", href = {slug_url} => "["{&slug}"]"))))
          (html!(html_entry_header(author, start_date, end_date, vec![]))))
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        return self.0.get(key);
    }

    pub fn texon(&self) -> Option<&String> {
        return self.0.get("taxon");
    }

    pub fn title(&self) -> Option<&String> {
        return self.0.get("title");
    }
}

pub struct HtmlEntry {
    pub metadata: EntryMetaData,
    pub catalog: Vec<(String, String)>,
    pub content: String,
}

impl HtmlEntry {
    pub fn get(&self, key: &str) -> Option<&String> {
        return self.metadata.get(key);
    }
}
