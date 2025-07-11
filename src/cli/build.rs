// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use eyre::{eyre, WrapErr};
use std::{fs, path::PathBuf};

use crate::{
    assets_sync, compiler, config::{
        self, output_path, AssetsDir, BaseUrl, CompileConfig, FooterMode, OutputDir, RootDir,
        TreesDir,
    }, config_toml, html_flake
};

#[derive(clap::Args)]
pub struct BuildCommand {

    /// Path to the configuration file (e.g., "kodama.toml").
    #[arg(short, long, default_value_t = ("./kodama.toml").into())]
    config: String, 

    // /// Base URL or publish URL (e.g. https://www.example.com/)
    // #[arg(short, long, default_value_t = BaseUrl::default().0)]
    // base: String,

    // /// Path to the trees directory
    // ///
    // /// This directory contains the source files for your notes.
    // /// In all cases, kodama will ignore folders in this directory that start with `.` or `_`.
    // #[arg(short, long, default_value_t = TreesDir::default().0)]
    // trees: String,

    // /// Path to output directory
    // #[arg(short, long, default_value_t = OutputDir::default().0)]
    // output: String,

    // /// Path to assets directory relative to the output directory
    // #[arg(long, default_value_t = AssetsDir::default().0)]
    // assets: String,

    // /// Configures the project root (for absolute paths)
    // #[arg(short, long, default_value_t = RootDir::default().0)]
    // root: String,

    // /// Disable pretty urls (`/page` to `/page.html`)
    // #[arg(short, long)]
    // disable_pretty_urls: bool,

    // /// Hide parents part in slug (e.g. `tutorials/install` to `install`)
    // #[arg(short, long)]
    // short_slug: bool,

    // /// Specify the inline mode for the footer sections
    // #[arg(short, long, default_value_t)]
    // footer_mode: FooterMode,

    // /// Disable exporting the `*.css` file to the output directory
    // #[arg(long)]
    // disable_export_css: bool,

    // /// Display URL redirect links prepared for the editor (e.g. `vscode://file:`)
    // #[arg(long)]
    // edit: Option<String>,
}

pub fn compile(command: &BuildCommand) -> eyre::Result<()> {
    config_toml::apply_config(PathBuf::from(command.config.clone()))?;

    println!("{:#?}", config::CONFIG_TOML);

    // let root = &compile_command.root;
    // let _ = config::CONFIG.set(CompileConfig::new(
    //     config::RootDir(root.to_string()),
    //     config::TreesDir(compile_command.trees.to_string()),
    //     config::OutputDir(compile_command.output.to_string()),
    //     config::AssetsDir(compile_command.assets.to_string()),
    //     config::BaseUrl(compile_command.base.to_string()),
    //     compile_command.disable_pretty_urls,
    //     compile_command.short_slug,
    //     compile_command.footer_mode.clone(),
    //     compile_command.disable_export_css,
    //     compile_command.edit.clone(),
    // ));

    // match &compile_command.edit {
    //     Some(s) => println!("[{}] EDIT MODE IS ENABLE. Please note that your disk file path information will be included in the pages!", s),
    //     None => (),
    // }

    // if !compile_command.disable_export_css {
    //     export_css_files().wrap_err("Failed to export CSS")?;
    // }

    // compiler::compile_all(root).wrap_err_with(|| eyre!("Failed to compile project `{root}`"))?;
    // sync_assets_dir()?;

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

fn sync_assets_dir() -> eyre::Result<bool> {
    let source = config::root_dir().join(config::assets_dir());
    let target = config::output_dir().join(config::assets_dir());
    assets_sync::sync_assets(source, target)?;
    Ok(true)
}
