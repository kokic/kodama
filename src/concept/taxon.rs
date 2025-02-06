use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Taxon {
    pub numbering: Option<String>,
    pub text: String,
}

impl std::fmt::Debug for Taxon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("\"{}\"", self.display()))
    }
}

impl Taxon {
    pub fn new(numbering: Option<String>, text: String) -> Taxon {
        Taxon { numbering, text }
    }

    pub fn display(&self) -> String {
        match &self.numbering {
            Some(numbering) => {
                let text = match self.text.ends_with(". ") {
                    true => &self.text[0..self.text.len() - 2],
                    false => &self.text,
                };
                format!("{} {} ", text, numbering)
            }
            None => self.text.to_string(),
        }
    }
}
