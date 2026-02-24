// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{fs, sync::OnceLock};

use camino::Utf8Path;
use eyre::{eyre, WrapErr};

use crate::{
    assets_sync,
    compiler::{self, all_trees_source, DirtySet},
    config,
    environment::{self, output_path, BuildMode},
    html_flake,
};

#[derive(clap::Args)]
pub struct BuildCommand {
    /// Path to the configuration file (e.g., "Kodama.toml").
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG_PATH.into())]
    config: String,

    /// Enable verbose output.
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Enable verbose skip output.
    #[arg(long, default_value_t = false)]
    verbose_skip: bool,

    /// Rebuild all files, ignoring any caches.
    #[arg(visible_alias = "nc", long, default_value_t = false)]
    no_cache: bool,

    /// Skip generating `kodama.json`.
    #[arg(long, default_value_t = false)]
    no_indexes: bool,

    /// Skip generating `kodama.graph.json`.
    #[arg(long, default_value_t = false)]
    no_graph: bool,
}

static VERBOSE: OnceLock<bool> = OnceLock::new();
static VERBOSE_SKIP: OnceLock<bool> = OnceLock::new();
static NO_CACHE: OnceLock<bool> = OnceLock::new();

#[derive(Clone, Copy)]
pub struct BuildOptions {
    pub verbose: bool,
    pub verbose_skip: bool,
    pub no_cache: bool,
    pub outputs: compiler::CompileOutputs,
}

pub fn verbose() -> &'static bool {
    VERBOSE.get().unwrap_or(&false)
}

pub fn verbose_skip() -> &'static bool {
    VERBOSE_SKIP.get().unwrap_or(&false)
}

pub fn enable_no_cache() -> &'static bool {
    NO_CACHE.get().unwrap_or(&false)
}

/// This function invoked the [`environment::init_environment`] function to initialize the environment
pub fn build(command: &BuildCommand) -> eyre::Result<()> {
    build_with(
        &command.config,
        BuildMode::Build,
        BuildOptions {
            verbose: command.verbose,
            verbose_skip: command.verbose_skip,
            no_cache: command.no_cache,
            outputs: compiler::CompileOutputs {
                indexes: !command.no_indexes,
                graph: !command.no_graph,
            },
        },
    )
}

pub fn build_with(config: &str, mode: BuildMode, options: BuildOptions) -> eyre::Result<()> {
    build_with_dirty(config, mode, options, None)
}

pub fn build_with_dirty(
    config: &str,
    mode: BuildMode,
    options: BuildOptions,
    dirty_paths: Option<&DirtySet>,
) -> eyre::Result<()> {
    environment::init_environment(config.into(), mode)?;
    _ = VERBOSE.set(options.verbose);
    _ = VERBOSE_SKIP.set(options.verbose_skip);
    _ = NO_CACHE.set(options.no_cache);

    if !environment::inline_css() {
        export_css_files().wrap_err("failed to export CSS")?;
    }

    let root = environment::root_dir();
    let workspace = all_trees_source(&environment::trees_dir(), dirty_paths)?;
    let expanded_dirty = dirty_paths.map(|paths| compiler::expand_dirty_paths(&workspace, paths));
    compiler::compile(workspace, expanded_dirty.as_ref(), options.outputs).wrap_err_with(|| {
        let root_display = root
            .canonicalize()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| root.as_str().to_string());
        eyre!("failed to compile site `{}`", root_display)
    })?;

    sync_assets_dir()?;

    Ok(())
}

fn export_css_files() -> eyre::Result<()> {
    export_css_file(html_flake::html_main_style(), "main.css")?;
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
    let asset_dir = environment::assets_dir();
    let asset_name = asset_dir
        .file_name()
        .ok_or_else(|| eyre!("invalid assets directory path: {}", asset_dir))?;
    let target = environment::output_dir().join(asset_name);

    assets_sync::sync_assets(asset_dir, target)
}
