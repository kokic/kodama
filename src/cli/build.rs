// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::fs;

use camino::Utf8Path;
use eyre::{eyre, WrapErr};

use crate::{
    assets_sync,
    compiler::{self, all_trees_source},
    config::{self, output_path, BuildMode},
    config_toml, html_flake,
};

#[derive(clap::Args)]
pub struct BuildCommand {
    /// Path to the configuration file (e.g., "kodama.toml").
    #[arg(short, long, default_value_t = config_toml::DEFAULT_CONFIG_PATH.into())]
    config: String,
}

/// This function invoked the [`config::init_environment`] function to initialize the environment]
pub fn build(command: &BuildCommand) -> eyre::Result<()> {
    build_with(&command.config, BuildMode::Build)
}

pub fn build_with(config: &str, mode: BuildMode) -> eyre::Result<()> {
    config::init_environment(config.into(), mode)?;

    if !config::inline_css() {
        export_css_files().wrap_err("failed to export CSS")?;
    }

    let root = config::root_dir();
    let workspace = all_trees_source(&config::trees_dir())?;
    compiler::compile(workspace).wrap_err_with(|| {
        eyre!(
            "failed to compile site `{}`",
            root.canonicalize().unwrap().display()
        )
    })?;

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
    let path = Utf8Path::new(&path);
    if !path.exists() {
        fs::write(path, css_content)
            .wrap_err_with(|| eyre!("failed to write CSS file to \"{}\"", path))?;
    }
    Ok(())
}

/// Synchronize the assets directory [`config::assets_dir`] with the
/// output directory [`config::output_dir()`].
fn sync_assets_dir() -> eyre::Result<bool> {
    let asset_dir = config::assets_dir();
    let target = config::output_dir().join(asset_dir.file_name().unwrap());

    assets_sync::sync_assets(asset_dir, target)?;
    Ok(true)
}
