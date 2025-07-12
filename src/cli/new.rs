use std::path::PathBuf;

use clap::Parser;
use eyre::Context;

use crate::config_toml;

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
        return Err(eyre::eyre!(
            "Site path already exists: {}",
            site_path.display()
        ));
    }

    std::fs::create_dir_all(site_path).wrap_err("Failed to create site directory")?;
    println!("Created new site at: {}", site_path.display());

    // Create default config file in the new site directory
    let config_command = NewConfigCommand {
        path: config_toml::DEFAULT_CONFIG_PATH.into(),
    };
    crate::cli::new::new_config(&config_command)?;

    Ok(())
}

#[derive(clap::Args)]
pub struct NewConfigCommand {
    /// Path to the new configuration file.
    #[arg(default_value_t = config_toml::DEFAULT_CONFIG_PATH.into())]
    pub path: String,
}

pub fn new_config(command: &NewConfigCommand) -> eyre::Result<()> {
    let config = config_toml::Config::default();
    let toml = toml::to_string(&config).wrap_err("Failed to serialize default config")?;

    let config_path = &command.path;
    std::fs::write(config_path, toml).wrap_err("Failed to create default config file")?;
    println!("Created new config at: {}", config_path);
    Ok(())
}

pub const DEFAULT_TEMPLATE: &str = "./template";

#[derive(clap::Args)]
pub struct NewSectionCommand {
    /// Path to the new section.
    #[arg(required = true)]
    pub path: PathBuf,

    /// Path to the template file to use for the new section.
    #[arg(short, long, default_value_t = DEFAULT_TEMPLATE.to_string())]
    pub template: String,
}

pub fn new_section(command: &NewSectionCommand) -> eyre::Result<()> {
    let template = &command.template;
    let default_not_exists = template == DEFAULT_TEMPLATE && !std::fs::exists(&template)?;

    let template = if default_not_exists {
        String::new()
    } else {
        std::fs::read_to_string(&template)
            .map_err(|e| eyre::eyre!("Failed to read template file: {}", e))?
            .replace(
                "<FILE_NAME>",
                &command.path.file_stem().unwrap().to_str().unwrap(),
            )
    };

    std::fs::write(&command.path, template)
        .map_err(|e| eyre::eyre!("Failed to create section file: {}", e))?;

    println!("Created new section at: {}", command.path.display());

    Ok(())
}
