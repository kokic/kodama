// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{
    fs,
    io::Write,
    sync::{
        atomic::{AtomicU64, Ordering},
        OnceLock,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use camino::Utf8Path;
use eyre::{eyre, WrapErr};

use crate::{
    assets_sync,
    cli::output::OutputControlArgs,
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

    #[command(flatten)]
    output: OutputControlArgs,
}

static VERBOSE: OnceLock<bool> = OnceLock::new();
static VERBOSE_SKIP: OnceLock<bool> = OnceLock::new();
static NO_CACHE: OnceLock<bool> = OnceLock::new();
static RELOAD_MARKER_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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

pub fn no_cache_enabled() -> &'static bool {
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
            outputs: command.output.resolve(compiler::CompileOutputs::default()),
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
    environment::ensure_cache_version()?;
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
    write_reload_marker(mode)?;

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

fn write_reload_marker(mode: BuildMode) -> eyre::Result<()> {
    if !matches!(mode, BuildMode::Serve) {
        return Ok(());
    }

    let output_dir = environment::output_dir();
    let marker_path = environment::reload_marker_path(output_dir.as_path());
    write_reload_marker_atomically(marker_path.as_path(), &next_reload_marker_stamp())?;
    Ok(())
}

fn next_reload_marker_stamp() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = RELOAD_MARKER_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{nanos}-{sequence}")
}

fn write_reload_marker_atomically(marker_path: &Utf8Path, stamp: &str) -> eyre::Result<()> {
    environment::create_parent_dirs(marker_path);
    let parent = marker_path.parent().ok_or_else(|| {
        eyre!(
            "failed to resolve parent directory for reload marker `{}`",
            marker_path
        )
    })?;
    let filename = marker_path.file_name().ok_or_else(|| {
        eyre!(
            "failed to resolve filename for reload marker `{}`",
            marker_path
        )
    })?;
    let temp_filename = format!("{filename}.tmp.{}.{}", std::process::id(), stamp);
    let temp_path = parent.join(temp_filename);

    let write_result = (|| -> eyre::Result<()> {
        let mut file = fs::File::create(temp_path.as_std_path())
            .wrap_err_with(|| eyre!("failed to create temp reload marker `{}`", temp_path))?;
        file.write_all(stamp.as_bytes())
            .wrap_err_with(|| eyre!("failed to write temp reload marker `{}`", temp_path))?;
        file.sync_all()
            .wrap_err_with(|| eyre!("failed to sync temp reload marker `{}`", temp_path))?;
        Ok(())
    })();

    if let Err(err) = write_result {
        let _ = fs::remove_file(temp_path.as_std_path());
        return Err(err);
    }

    fs::rename(temp_path.as_std_path(), marker_path.as_std_path()).wrap_err_with(|| {
        eyre!(
            "failed to atomically replace reload marker `{}` from `{}`",
            marker_path,
            temp_path
        )
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;

    #[test]
    fn test_next_reload_marker_stamp_is_unique() {
        let a = next_reload_marker_stamp();
        let b = next_reload_marker_stamp();
        assert_ne!(a, b);
    }

    #[test]
    fn test_write_reload_marker_atomically_overwrites() {
        let base = std::env::temp_dir().join(format!("kodama-reload-marker-{}", fastrand::u64(..)));
        let base = Utf8PathBuf::from_path_buf(base).unwrap();
        let marker = base.join("serve/kodama.reload");

        write_reload_marker_atomically(marker.as_path(), "v1").unwrap();
        let first = fs::read_to_string(marker.as_std_path()).unwrap();
        assert_eq!(first, "v1");

        write_reload_marker_atomically(marker.as_path(), "v2").unwrap();
        let second = fs::read_to_string(marker.as_std_path()).unwrap();
        assert_eq!(second, "v2");

        let _ = fs::remove_dir_all(base.as_std_path());
    }
}
