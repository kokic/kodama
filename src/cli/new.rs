// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use std::path::PathBuf;

use clap::Parser;
use eyre::Context;

use crate::{config, config_toml};

#[derive(Parser)]
pub struct NewCommandCli {
    #[command(subcommand)]
    pub command: NewCommand,
}

#[derive(clap::Subcommand)]
pub enum NewCommand {
    /// Create a new kodama site.
    Site(NewSiteCommand),

    /// Create a new config file.
    Config(NewConfigCommand),

    /// Create a new section.
    #[command(visible_alias = "post")]
    Section(NewSectionCommand),
}

#[derive(clap::Args)]
pub struct NewSiteCommand {
    /// Path to the new site.
    #[arg(required = true)]
    pub path: PathBuf,
}

pub fn new_site(command: &NewSiteCommand) -> eyre::Result<()> {
    let site_path = &command.path;
    if site_path.exists() {
        return Err(eyre::eyre!("Already exists: {}", site_path.display()));
    }

    std::fs::create_dir_all(site_path).wrap_err("Failed to create site directory")?;
    println!("Created new site at: {}", site_path.display());

    let default_config_path = site_path.join(config_toml::DEFAULT_CONFIG_PATH);
    let default_source_dir = site_path.join(config_toml::DEFAULT_SOURCE_DIR);

    // Create default config file in the new site directory
    new_config_inner(&default_config_path)?;

    // Create the `index.md` section in the new site directory
    new_section_inner(
        &PathBuf::from(DEFAULT_SECTION_PATH),
        DEFAULT_TEMPLATE,
        &default_config_path,
    )?;

    // Create the default source directory `trees`
    std::fs::create_dir(default_source_dir)
        .wrap_err("Failed to create default source directory")?;

    Ok(())
}

#[derive(clap::Args)]
pub struct NewConfigCommand {
    /// Path to the new configuration file.
    #[arg(default_value_t = config_toml::DEFAULT_CONFIG_PATH.into())]
    pub path: String,
}

pub fn new_config(command: &NewConfigCommand) -> eyre::Result<()> {
    new_config_inner(&PathBuf::from(&command.path))
}

fn new_config_inner(config_path: &PathBuf) -> Result<(), eyre::Error> {
    let config = config_toml::Config::default();
    let toml = toml::to_string(&config).wrap_err("Failed to serialize default config")?;

    std::fs::write(config_path, toml).wrap_err("Failed to create default config file")?;
    println!("Created new config at: {}", config_path.display());
    Ok(())
}

pub const DEFAULT_SECTION_PATH: &str = "./index.md";
pub const DEFAULT_SECTION_PATH_TYPST: &str = "./index.typ";

pub const DEFAULT_TEMPLATE: &str = "./template";
pub const DEFAULT_TEMPLATE_CONTENT: &str = r#"
---
title: <FILE_NAME>
---
"#;

#[derive(clap::Args)]
pub struct NewSectionCommand {
    /// Path to the new section.
    #[arg(required = true)]
    pub path: PathBuf,

    /// Path to the template file to use for the new section.
    #[arg(short, long, default_value_t = DEFAULT_TEMPLATE.to_string())]
    pub template: String,

    /// Path to the configuration file (e.g., "kodama.toml").
    #[arg(short, long, default_value_t = config_toml::DEFAULT_CONFIG_PATH.into())]
    pub config: String,
}

/// This function invoked the [`config_toml::apply_config`] function to apply the configuration.
pub fn new_section(command: &NewSectionCommand) -> eyre::Result<()> {
    new_section_inner(
        &command.path,
        &command.template,
        &PathBuf::from(&command.config),
    )
}

/// This function invoked the [`config_toml::apply_config`] function to apply the configuration.
fn new_section_inner(path: &PathBuf, template: &str, config: &PathBuf) -> eyre::Result<()> {
    config_toml::apply_config(PathBuf::from(config))?;

    let default_not_exists = template == DEFAULT_TEMPLATE && !std::fs::exists(template)?;

    let content = if default_not_exists {
        DEFAULT_TEMPLATE_CONTENT.to_string()
    } else {
        std::fs::read_to_string(template)
            .map_err(|e| eyre::eyre!("Failed to read template file: {}", e))?
    };

    let filestem = path.file_stem().unwrap().to_str().unwrap();
    let content = content.replace("<FILE_NAME>", filestem);

    let section_path = config::root_dir().join(path);
    let section_path_display = section_path.display();

    if section_path.exists() {
        return Err(eyre::eyre!("Already exists: {}", section_path_display));
    } else {
        std::fs::create_dir_all(&section_path.parent().unwrap())
            .map_err(|e| eyre::eyre!("Failed to create section directory: {}", e))?;
    }

    std::fs::write(&section_path, content)
        .map_err(|e| eyre::eyre!("Failed to create section file: {}", e))?;
    println!("Created new section at: {}", section_path_display);

    Ok(())
}
