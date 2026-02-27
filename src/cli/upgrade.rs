// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use camino::Utf8PathBuf;
use eyre::{eyre, WrapErr};

use crate::config;

#[derive(clap::Args)]
pub struct UpgradeCommand {
    /// Path to the source configuration file (e.g., "Kodama.toml").
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG_PATH.into())]
    pub config: String,

    /// Output path of the upgraded configuration file.
    /// Defaults to overwriting the source file.
    #[arg(short, long)]
    pub output: Option<String>,
}

pub fn upgrade(command: &UpgradeCommand) -> eyre::Result<()> {
    let source_path =
        config::find_config(Utf8PathBuf::from(&command.config)).wrap_err_with(|| {
            eyre!(
                "failed to locate configuration file from \"{}\"",
                command.config
            )
        })?;
    let source = std::fs::read_to_string(&source_path)
        .wrap_err_with(|| eyre!("failed to read config file \"{}\"", source_path))?;
    let (upgraded, upgraded_config) = upgrade_content(&source)?;

    let output_path = command
        .output
        .as_ref()
        .map(Utf8PathBuf::from)
        .unwrap_or_else(|| source_path.clone());
    std::fs::write(&output_path, upgraded)
        .wrap_err_with(|| eyre!("failed to write upgraded config to \"{}\"", output_path))?;
    sync_kodama_typ(output_path.as_path(), &upgraded_config)?;

    if output_path == source_path {
        println!("Upgraded config at: {}", output_path);
    } else {
        println!(
            "Upgraded config from \"{}\" to \"{}\"",
            source_path, output_path
        );
    }
    println!(
        "Synced Typst library: {}",
        trees_lib_kodama_typ_path(output_path.as_path(), &upgraded_config)
    );
    Ok(())
}

fn upgrade_content(source: &str) -> eyre::Result<(String, config::Config)> {
    let config = config::parse_config(source)?;
    let mut upgraded =
        toml::to_string(&config).wrap_err("failed to serialize upgraded configuration")?;
    if !upgraded.ends_with('\n') {
        upgraded.push('\n');
    }
    Ok((upgraded, config))
}

fn trees_lib_kodama_typ_path(
    config_path: &camino::Utf8Path,
    config: &config::Config,
) -> Utf8PathBuf {
    let root = config_path
        .parent()
        .map(|p| p.to_owned())
        .unwrap_or_else(|| Utf8PathBuf::from("."));
    root.join(&config.kodama.trees)
        .join("_lib")
        .join("kodama.typ")
}

fn sync_kodama_typ(config_path: &camino::Utf8Path, config: &config::Config) -> eyre::Result<()> {
    let typ_path = trees_lib_kodama_typ_path(config_path, config);
    let parent = typ_path.parent().ok_or_else(|| {
        eyre!(
            "failed to resolve parent directory for Typst library path \"{}\"",
            typ_path
        )
    })?;
    std::fs::create_dir_all(parent)
        .wrap_err_with(|| eyre!("failed to create Typst library directory \"{}\"", parent))?;
    std::fs::write(&typ_path, include_str!("../include/kodama.typ"))
        .wrap_err_with(|| eyre!("failed to sync Typst library file \"{}\"", typ_path))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upgrade_content_rewrites_into_current_shape() {
        let source = r#"
[kodama]
base-url = "https://example.com/"

[build]
output = "./dist"
"#;
        let (upgraded, _) = upgrade_content(source).unwrap();
        assert!(upgraded.contains("[kodama]"));
        assert!(upgraded.contains("base-url = \"https://example.com/\""));
        assert!(upgraded.contains("[toc]"));
        assert!(upgraded.contains("[text]"));
        assert!(upgraded.contains("[build]"));
        assert!(upgraded.contains("output = \"./dist\""));
        assert!(upgraded.contains("[serve]"));
        assert!(upgraded.contains("command = ["));
    }

    #[test]
    fn test_upgrade_content_output_is_parseable() {
        let (upgraded, config) = upgrade_content("").unwrap();
        let parsed = config::parse_config(&upgraded).unwrap();
        assert_eq!(parsed.kodama.trees, "trees");
        assert_eq!(parsed.build.output, "./publish");
        assert_eq!(parsed.serve.output, "./.cache/publish");
        assert_eq!(config.kodama.trees, "trees");
    }

    #[test]
    fn test_trees_lib_kodama_typ_path_uses_config_directory_and_trees_setting() {
        let mut cfg = config::Config::default();
        cfg.kodama.trees = "notes".to_string();

        let path = trees_lib_kodama_typ_path(camino::Utf8Path::new("D:/site/Kodama.toml"), &cfg);
        assert_eq!(path, Utf8PathBuf::from("D:/site/notes/_lib/kodama.typ"));
    }

    #[test]
    fn test_sync_kodama_typ_creates_and_overwrites_library_file() {
        let root = crate::test_io::case_dir("upgrade-kodama-typ");
        let config_path = root.join("Kodama.toml");
        let mut cfg = config::Config::default();
        cfg.kodama.trees = "trees".to_string();

        let typ_path = trees_lib_kodama_typ_path(config_path.as_path(), &cfg);
        std::fs::create_dir_all(
            typ_path
                .parent()
                .expect("kodama.typ path should have parent")
                .as_std_path(),
        )
        .unwrap();
        std::fs::write(typ_path.as_std_path(), "OLD").unwrap();

        sync_kodama_typ(config_path.as_path(), &cfg).unwrap();
        let content = std::fs::read_to_string(typ_path.as_std_path()).unwrap();
        assert_eq!(content, include_str!("../include/kodama.typ"));

        let _ = std::fs::remove_dir_all(root.as_std_path());
    }

    #[test]
    fn test_upgrade_writes_config_and_syncs_kodama_typ() {
        let root = crate::test_io::case_dir("upgrade-command");
        std::fs::create_dir_all(root.as_std_path()).unwrap();
        let source_config = root.join("Kodama.toml");
        std::fs::write(
            source_config.as_std_path(),
            r#"
[kodama]
trees = "content"
"#,
        )
        .unwrap();

        upgrade(&UpgradeCommand {
            config: source_config.to_string(),
            output: None,
        })
        .unwrap();

        let upgraded = std::fs::read_to_string(source_config.as_std_path()).unwrap();
        assert!(upgraded.contains("[kodama]"));
        assert!(upgraded.contains("trees = \"content\""));
        let typ_path = root.join("content/_lib/kodama.typ");
        let typ_content = std::fs::read_to_string(typ_path.as_std_path()).unwrap();
        assert_eq!(typ_content, include_str!("../include/kodama.typ"));

        let _ = std::fs::remove_dir_all(root.as_std_path());
    }
}
