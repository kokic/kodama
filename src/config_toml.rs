use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::{self, FooterMode};

pub const DEFAULT_CONFIG_PATH: &'static str = "./Kodama.toml";
pub const DEFAULT_SOURCE_DIR: &'static str = "trees";
pub const DEFAULT_ASSETS_DIR: &'static str = "assets";

#[derive(Deserialize, Debug, Default, Serialize)]
pub struct Config {
    #[serde(default)]
    pub kodama: Kodama,

    #[serde(default)]
    pub build: Build,

    #[serde(default)]
    pub serve: Serve,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(default)]
pub struct Kodama {
    pub trees: String,
    pub assets: String,
    pub url: String,
}

impl Default for Kodama {
    fn default() -> Self {
        Self {
            trees: DEFAULT_SOURCE_DIR.to_string(),
            assets: DEFAULT_ASSETS_DIR.to_string(),
            url: "/".to_string(),
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
    pub output: String,
}

impl Default for Build {
    fn default() -> Self {
        Self {
            typst_root: "./".to_string(),
            short_slug: false,
            pretty_urls: false,
            footer_mode: FooterMode::default(),
            inline_css: false,
            output: "./publish".to_string(),
        }
    }
}

#[derive(Deserialize, Debug, Serialize)]
pub struct Serve {
    pub edit: Option<String>,
    pub output: String,
}

impl Default for Serve {
    fn default() -> Self {
        Self {
            edit: Some("vscode://file/".to_string()),
            output: "./.cache/publish".to_string(),
        }
    }
}

fn parse_config(config: &str) -> eyre::Result<Config> {
    let config: Config =
        toml::from_str(&config).map_err(|e| eyre::eyre!("Failed to parse config file: {}", e))?;
    Ok(config)
}

pub fn apply_config(toml_file: PathBuf) -> eyre::Result<()> {
    let root = toml_file
        .parent()
        .expect("Path terminates in a root or prefix!");
    let toml = std::fs::read_to_string(&toml_file)?;

    let _ = config::ROOT.set(root.to_path_buf());
    let _ = config::TOML.set(toml_file.file_name().unwrap().to_str().unwrap().to_string());
    let _ = config::CONFIG_TOML.set(parse_config(&toml)?);
    Ok(())
}

mod test {

    #[test]
    fn test_empty_toml() {
        let config = crate::config_toml::parse_config("").unwrap();
        assert_eq!(config.kodama.trees, "trees");
        assert_eq!(config.kodama.assets, "assets");
        assert_eq!(config.kodama.url, "/");
        assert_eq!(config.build.short_slug, false);
        assert_eq!(config.build.pretty_urls, false);
        assert_eq!(config.build.inline_css, false);
        assert_eq!(config.serve.edit, None);
    }

    #[test]
    fn test_simple_toml() {
        let config = crate::config_toml::parse_config(
            r#"
            [kodama]
            trees = ["source"]
            assets = ["assets", "static"]
            url = "https://example.com/"

            [build]
            short-slug = true
            inline-css = true
            "#,
        )
        .unwrap();

        println!("{:#?}", config)
    }
}
