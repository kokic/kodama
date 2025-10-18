// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

pub mod build;
pub mod kodama;
pub mod serve;
pub mod text;
pub mod toc;

use build::Build;
use camino::Utf8PathBuf;
use kodama::Kodama;
use serde::{Deserialize, Serialize};
use serve::Serve;
use text::Text;
use toc::Toc;

pub const DEFAULT_CONFIG_PATH: &str = "./Kodama.toml";

#[derive(Deserialize, Debug, Default, Serialize)]
pub struct Config {
    #[serde(default)]
    pub kodama: Kodama,

    #[serde(default)]
    pub toc: Toc,

    #[serde(default)]
    pub text: Text,

    #[serde(default)]
    pub build: Build,

    #[serde(default)]
    pub serve: Serve,
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
