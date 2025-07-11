use std::path::PathBuf;

use serde::Deserialize;

use crate::config::{self, CompileConfig, FooterMode};

#[derive(Deserialize, Debug)]
pub struct Config {
    #[serde(default)]
    kodama: Kodama,

    #[serde(default)]
    build: Build,

    #[serde(default)]
    serve: Serve,
}

#[derive(Deserialize, Debug)]
#[serde(default)]
struct Kodama {
    trees: Vec<String>,
    assets: Vec<String>,
    url: Option<String>,
}

impl Default for Kodama {
    fn default() -> Self {
        Self {
            trees: vec!["trees".to_string()],
            assets: vec!["assets".to_string()],
            url: None,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(default, rename_all = "kebab-case")]
struct Build {
    typst_root: String,
    short_slug: bool,
    pretty_urls: bool,
    footer_mode: FooterMode,
    inline_css: bool,
    output: String,
}

impl Default for Build {
    fn default() -> Self {
        Self {
            typst_root: "./".to_string(),
            short_slug: false,
            pretty_urls: false,
            footer_mode: FooterMode::default(),
            inline_css: false,
            output: ".cache/publish".to_string(),
        }
    }
}

#[derive(Deserialize, Debug)]
struct Serve {
    edit: Option<String>,
}

impl Default for Serve {
    fn default() -> Self {
        Self { edit: None }
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
        assert_eq!(config.kodama.trees, vec!["trees".to_string()]);
        assert_eq!(config.kodama.assets, vec!["assets".to_string()]);
        assert_eq!(config.kodama.url, None);
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
