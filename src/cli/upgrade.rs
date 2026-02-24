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
    let upgraded = upgrade_content(&source)?;

    let output_path = command
        .output
        .as_ref()
        .map(Utf8PathBuf::from)
        .unwrap_or_else(|| source_path.clone());
    std::fs::write(&output_path, upgraded)
        .wrap_err_with(|| eyre!("failed to write upgraded config to \"{}\"", output_path))?;

    if output_path == source_path {
        println!("Upgraded config at: {}", output_path);
    } else {
        println!(
            "Upgraded config from \"{}\" to \"{}\"",
            source_path, output_path
        );
    }
    Ok(())
}

fn upgrade_content(source: &str) -> eyre::Result<String> {
    let config = config::parse_config(source)?;
    let mut upgraded =
        toml::to_string(&config).wrap_err("failed to serialize upgraded configuration")?;
    if !upgraded.ends_with('\n') {
        upgraded.push('\n');
    }
    Ok(upgraded)
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
        let upgraded = upgrade_content(source).unwrap();
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
        let upgraded = upgrade_content("").unwrap();
        let parsed = config::parse_config(&upgraded).unwrap();
        assert_eq!(parsed.kodama.trees, "trees");
        assert_eq!(parsed.build.output, "./publish");
        assert_eq!(parsed.serve.output, "./.cache/publish");
    }
}
