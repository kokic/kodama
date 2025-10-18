use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Build {
    pub typst_root: String,
    pub short_slug: bool,
    pub pretty_urls: bool,
    pub footer_mode: FooterMode,
    pub inline_css: bool,
    pub asref: bool,
    pub output: String,
    pub edit: Option<String>,
}

impl Default for Build {
    fn default() -> Self {
        Self {
            typst_root: "trees".to_string(),
            short_slug: false,
            pretty_urls: false,
            footer_mode: FooterMode::default(),
            inline_css: false,
            asref: false,
            output: "./publish".to_string(),
            edit: None,
        }
    }
}

#[derive(Debug, Copy, Clone, clap::ValueEnum, Default, Deserialize, Serialize)]
pub enum FooterMode {
    #[default]
    #[serde(rename = "link")]
    Link,

    #[serde(rename = "embed")]
    Embed,
}

#[derive(Debug)]
pub struct ParseFooterModeError;

impl FromStr for FooterMode {
    type Err = ParseFooterModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "link" => Ok(FooterMode::Link),
            "embed" => Ok(FooterMode::Embed),
            _ => Err(ParseFooterModeError),
        }
    }
}

impl std::fmt::Display for FooterMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FooterMode::Link => write!(f, "link"),
            FooterMode::Embed => write!(f, "embed"),
        }
    }
}
