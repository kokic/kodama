// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use eyre::{eyre, WrapErr};
use std::{fs, path::PathBuf};

use crate::{
    assets_sync,
    compiler::{self, all_source_files},
    config::{self, output_path},
    config_toml, html_flake,
};

#[derive(clap::Args)]
pub struct BuildCommand {
    /// Path to the configuration file (e.g., "kodama.toml").
    #[arg(short, long, default_value_t = config_toml::DEFAULT_CONFIG_PATH.into())]
    config: String,
}

pub fn compile(command: &BuildCommand) -> eyre::Result<()> {
    config_toml::apply_config(PathBuf::from(command.config.clone()))?;

    // match config::editor_url() {
    //     Some(s) => println!("[{}] EDIT MODE IS ENABLE. Please note that your disk file path information will be included in the pages!", s),
    //     None => (),
    // }

    if config::inline_css() {
        export_css_files().wrap_err("Failed to export CSS")?;
    }

    let root = config::root_dir();
    let workspace = all_source_files(&config::trees_dir())?;
    compiler::compile(workspace)
        .wrap_err_with(|| eyre!("Failed to compile `{}`", root.display()))?;
    sync_assets_dir()?;

    Ok(())
}

fn export_css_files() -> eyre::Result<()> {
    export_css_file(html_flake::html_main_style(), "main.css")?;
    export_css_file(html_flake::html_typst_style(), "typst.css")?;
    Ok(())
}

fn export_css_file(css_content: &str, name: &str) -> eyre::Result<()> {
    let path = output_path(name);
    let path = std::path::Path::new(&path);
    if !path.exists() {
        fs::write(path, css_content)
            .wrap_err_with(|| eyre!("Failed to write CSS file to \"{}\"", path.display()))?;
    }
    Ok(())
}

/// Synchronize all assets directory [`config::assets_dir`] with the
/// output directory [`config::output_dir()`].
fn sync_assets_dir() -> eyre::Result<bool> {
    let assets: Vec<PathBuf> = config::assets_dir();

    for asset_dir in assets {
        let target = config::output_dir().join(&asset_dir);
        assets_sync::sync_assets(asset_dir, target)?;
    }

    Ok(true)
}
