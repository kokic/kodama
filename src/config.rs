// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::str::FromStr;

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

pub const DEFAULT_CONFIG_PATH: &str = "./Kodama.toml";
pub const DEFAULT_SOURCE_DIR: &str = "trees";
pub const DEFAULT_ASSETS_DIR: &str = "assets";
pub const DEFAULT_BASE_URL: &str = "/";

#[derive(Deserialize, Debug, Default, Serialize)]
pub struct Config {
    #[serde(default)]
    pub kodama: Kodama,

    #[serde(default)]
    pub toc: Toc,

    #[serde(default)]
    pub build: Build,

    #[serde(default)]
    pub serve: Serve,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Kodama {
    pub trees: String,
    pub assets: String,
    pub base_url: String,
}

impl Default for Kodama {
    fn default() -> Self {
        Self {
            trees: DEFAULT_SOURCE_DIR.to_string(),
            assets: DEFAULT_ASSETS_DIR.to_string(),
            base_url: DEFAULT_BASE_URL.to_string(),
        }
    }
}

#[derive(Debug, Copy, Clone, clap::ValueEnum, Default, Deserialize, Serialize)]
pub enum TocPlacement {
    #[serde(rename = "left")]
    Left,

    #[default]
    #[serde(rename = "right")]
    Right,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Toc {
    pub placement: TocPlacement,
    pub sticky: bool,
    pub mobile_sticky: bool, 
    pub max_width: String, 
}

impl Default for Toc {
    fn default() -> Self {
        Self {
            placement: TocPlacement::Right,
            sticky: true,
            mobile_sticky: true,
            max_width: "45ex".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct ParseTocPlacementError;

impl FromStr for TocPlacement {
    type Err = ParseTocPlacementError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "left" => Ok(TocPlacement::Left),
            "right" => Ok(TocPlacement::Right),
            _ => Err(ParseTocPlacementError),
        }
    }
}

impl std::fmt::Display for TocPlacement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TocPlacement::Left => write!(f, "left"),
            TocPlacement::Right => write!(f, "right"),
        }
    }
}

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
        }
    }
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Serve {
    pub edit: Option<String>,
    pub output: String,
    pub command: Vec<String>,
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

impl Default for Serve {
    fn default() -> Self {
        Self {
            edit: Some("vscode://file/".to_string()),
            output: "./.cache/publish".to_string(),
            command: [
                "miniserve",
                "<output>",
                "--index",
                "index.html",
                "--pretty-urls",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        }
    }
}

/// Try to find toml file in the current directory or the parent directory.
pub fn find_config(mut toml_file: Utf8PathBuf) -> eyre::Result<Utf8PathBuf> {
    if !toml_file.exists() {
        let parent = toml_file.parent().unwrap().canonicalize_utf8()?;
        let parent = parent.parent().unwrap();

        toml_file = parent.join(DEFAULT_CONFIG_PATH);
        if !toml_file.exists() {
            return Err(eyre::eyre!("cannot find configuration file: {}", toml_file));
        }
    }
    Ok(toml_file)
}

pub fn parse_config(config: &str) -> eyre::Result<Config> {
    let config: Config =
        toml::from_str(config).map_err(|e| eyre::eyre!("failed to parse config file: {}", e))?;
    Ok(config)
}

mod test {

    #[test]
    fn test_empty_toml() {
        let serve = crate::config::Serve::default();
        let config = crate::config::parse_config("").unwrap();

        assert_eq!(config.kodama.trees, "trees");
        assert_eq!(config.kodama.assets, "assets");
        assert_eq!(config.kodama.base_url, "/");
        assert!(!config.build.short_slug);
        assert!(!config.build.pretty_urls);
        assert!(!config.build.inline_css);
        assert_eq!(config.serve.edit, serve.edit);
        assert_eq!(config.serve.output, serve.output);
    }

    #[test]
    fn test_simple_toml() {
        let serve = crate::config::Serve::default();
        let config = crate::config::parse_config(
            r#"
            [kodama]
            trees = "source"
            assets = "assets"
            base-url = "https://example.com/"

            [build]
            short-slug = true
            inline-css = true
            "#,
        )
        .unwrap();

        assert_eq!(config.kodama.trees, "source");
        assert_eq!(config.kodama.assets, "assets");
        assert_eq!(config.kodama.base_url, "https://example.com/");
        assert!(config.build.short_slug);
        assert!(config.build.inline_css);
        assert_eq!(config.serve.edit, serve.edit);
        assert_eq!(config.serve.output, serve.output);
    }
}
