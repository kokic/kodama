// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use camino::Utf8PathBuf;
use eyre::{eyre, WrapErr};

use crate::config;

#[derive(clap::Args)]
pub struct UpgradeCommand {
    /// Optional subcommand. If omitted, behaves like `upgrade all`.
    #[command(subcommand)]
    pub command: Option<UpgradeSubcommand>,
}

#[derive(clap::Subcommand)]
pub enum UpgradeSubcommand {
    /// Upgrade config and sync Typst library files.
    #[command(visible_alias = "a")]
    All(UpgradeAllCommand),

    /// Upgrade config file only.
    #[command(visible_alias = "c")]
    Config(UpgradeConfigCommand),

    /// Sync trees/_lib/kodama.typ only.
    #[command(name = "typst-lib", visible_alias = "t")]
    TypstLib(UpgradeTypstLibCommand),
}

#[derive(clap::Args)]
pub struct UpgradeAllCommand {
    /// Path to the source configuration file (e.g., "Kodama.toml").
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG_PATH.into())]
    pub config: String,

    /// Output path of the upgraded configuration file.
    /// Defaults to overwriting the source file.
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct UpgradeConfigCommand {
    /// Path to the source configuration file (e.g., "Kodama.toml").
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG_PATH.into())]
    pub config: String,

    /// Output path of the upgraded configuration file.
    /// Defaults to overwriting the source file.
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(clap::Args)]
pub struct UpgradeTypstLibCommand {
    /// Path to the source configuration file (e.g., "Kodama.toml").
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG_PATH.into())]
    pub config: String,
}

pub fn upgrade(command: &UpgradeCommand) -> eyre::Result<()> {
    match &command.command {
        Some(UpgradeSubcommand::All(args)) => run_upgrade_all(args),
        Some(UpgradeSubcommand::Config(args)) => run_upgrade_config(args),
        Some(UpgradeSubcommand::TypstLib(args)) => run_upgrade_typst_lib(args),
        None => run_upgrade_all(&UpgradeAllCommand {
            config: config::DEFAULT_CONFIG_PATH.to_string(),
            output: None,
        }),
    }
}

fn run_upgrade_all(command: &UpgradeAllCommand) -> eyre::Result<()> {
    let upgraded = upgrade_config_file(&command.config, command.output.as_deref())?;
    print_config_upgrade_message(
        upgraded.source_path.as_path(),
        upgraded.output_path.as_path(),
    );
    let typ_path = sync_kodama_typ(upgraded.output_path.as_path(), &upgraded.config)?;
    println!("Synced Typst library: {}", typ_path);
    Ok(())
}

fn run_upgrade_config(command: &UpgradeConfigCommand) -> eyre::Result<()> {
    let upgraded = upgrade_config_file(&command.config, command.output.as_deref())?;
    print_config_upgrade_message(
        upgraded.source_path.as_path(),
        upgraded.output_path.as_path(),
    );
    Ok(())
}

fn run_upgrade_typst_lib(command: &UpgradeTypstLibCommand) -> eyre::Result<()> {
    let source_path = resolve_config_path(&command.config)?;
    let source = std::fs::read_to_string(&source_path)
        .wrap_err_with(|| eyre!("failed to read config file \"{}\"", source_path))?;
    let config = config::parse_config(&source).wrap_err_with(|| {
        eyre!(
            "failed to parse config file \"{}\" while syncing Typst library",
            source_path
        )
    })?;
    let typ_path = sync_kodama_typ(source_path.as_path(), &config)?;
    println!("Synced Typst library: {}", typ_path);
    Ok(())
}

fn print_config_upgrade_message(source_path: &camino::Utf8Path, output_path: &camino::Utf8Path) {
    if output_path == source_path {
        println!("Upgraded config at: {}", output_path);
    } else {
        println!(
            "Upgraded config from \"{}\" to \"{}\"",
            source_path, output_path
        );
    }
}

fn resolve_config_path(config_path: &str) -> eyre::Result<Utf8PathBuf> {
    config::find_config(Utf8PathBuf::from(config_path)).wrap_err_with(|| {
        eyre!(
            "failed to locate configuration file from \"{}\"",
            config_path
        )
    })
}

struct UpgradedConfig {
    source_path: Utf8PathBuf,
    output_path: Utf8PathBuf,
    config: config::Config,
}

fn upgrade_config_file(
    config_path: &str,
    output_path: Option<&str>,
) -> eyre::Result<UpgradedConfig> {
    let source_path = resolve_config_path(config_path)?;
    let source = std::fs::read_to_string(&source_path)
        .wrap_err_with(|| eyre!("failed to read config file \"{}\"", source_path))?;
    let (upgraded, upgraded_config) = upgrade_content(&source)?;

    let output_path = output_path
        .map(Utf8PathBuf::from)
        .unwrap_or_else(|| source_path.clone());
    std::fs::write(&output_path, upgraded)
        .wrap_err_with(|| eyre!("failed to write upgraded config to \"{}\"", output_path))?;
    Ok(UpgradedConfig {
        source_path,
        output_path,
        config: upgraded_config,
    })
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

fn sync_kodama_typ(
    config_path: &camino::Utf8Path,
    config: &config::Config,
) -> eyre::Result<Utf8PathBuf> {
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
    Ok(typ_path)
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

        let synced = sync_kodama_typ(config_path.as_path(), &cfg).unwrap();
        assert_eq!(synced, typ_path);
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
            command: Some(UpgradeSubcommand::All(UpgradeAllCommand {
                config: source_config.to_string(),
                output: None,
            })),
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

    #[test]
    fn test_upgrade_config_subcommand_only_writes_config() {
        let root = crate::test_io::case_dir("upgrade-config-only");
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
            command: Some(UpgradeSubcommand::Config(UpgradeConfigCommand {
                config: source_config.to_string(),
                output: None,
            })),
        })
        .unwrap();

        let upgraded = std::fs::read_to_string(source_config.as_std_path()).unwrap();
        assert!(upgraded.contains("[kodama]"));
        assert!(upgraded.contains("trees = \"content\""));
        assert!(!root.join("content/_lib/kodama.typ").exists());

        let _ = std::fs::remove_dir_all(root.as_std_path());
    }

    #[test]
    fn test_upgrade_typst_lib_subcommand_only_syncs_library() {
        let root = crate::test_io::case_dir("upgrade-typst-lib-only");
        std::fs::create_dir_all(root.as_std_path()).unwrap();
        let source_config = root.join("Kodama.toml");
        let source = r#"
[kodama]
trees = "content"
"#;
        std::fs::write(source_config.as_std_path(), source).unwrap();

        upgrade(&UpgradeCommand {
            command: Some(UpgradeSubcommand::TypstLib(UpgradeTypstLibCommand {
                config: source_config.to_string(),
            })),
        })
        .unwrap();

        let unchanged_config = std::fs::read_to_string(source_config.as_std_path()).unwrap();
        assert_eq!(unchanged_config, source);
        let typ_path = root.join("content/_lib/kodama.typ");
        let typ_content = std::fs::read_to_string(typ_path.as_std_path()).unwrap();
        assert_eq!(typ_content, include_str!("../include/kodama.typ"));

        let _ = std::fs::remove_dir_all(root.as_std_path());
    }
}
