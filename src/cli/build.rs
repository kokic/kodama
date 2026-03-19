// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{
    fs,
    io::Write,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
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
static ATOMIC_WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static SERVE_SESSION: OnceLock<Mutex<Option<compiler::ServeCompileSession>>> = OnceLock::new();

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

    export_static_files().wrap_err("failed to export static files")?;

    let root = environment::root_dir();
    let trees_dir = environment::trees_dir();
    let workspace = all_trees_source(&trees_dir)?;
    compiler::sync_typst_svg_assets(trees_dir.as_path(), dirty_paths)?;
    let expanded_dirty = dirty_paths.map(|paths| compiler::expand_dirty_paths(&workspace, paths));
    compile_with_mode(mode, workspace, expanded_dirty.as_ref(), options.outputs).wrap_err_with(
        || {
            let root_display = root
                .canonicalize()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| root.as_str().to_string());
            eyre!("failed to compile site `{}`", root_display)
        },
    )?;

    sync_assets_dir()?;
    write_reload_marker(mode)?;

    Ok(())
}

pub fn serve_rewrite_from_memory(config: &str, options: BuildOptions) -> eyre::Result<()> {
    environment::init_environment(config.into(), BuildMode::Serve)?;
    environment::ensure_cache_version()?;
    _ = VERBOSE.set(options.verbose);
    _ = VERBOSE_SKIP.set(options.verbose_skip);
    _ = NO_CACHE.set(options.no_cache);

    export_static_files().wrap_err("failed to export static files")?;

    rewrite_serve_with_session(options.outputs)?;
    sync_assets_dir()?;
    write_reload_marker(BuildMode::Serve)?;
    Ok(())
}

fn compile_with_mode(
    mode: BuildMode,
    workspace: compiler::Workspace,
    dirty_paths: Option<&DirtySet>,
    outputs: compiler::CompileOutputs,
) -> eyre::Result<()> {
    if matches!(mode, BuildMode::Serve) {
        return compile_serve_with_session(workspace, dirty_paths, outputs);
    }

    clear_serve_session();
    compiler::compile(workspace, dirty_paths, outputs)
}

fn serve_session_lock() -> &'static Mutex<Option<compiler::ServeCompileSession>> {
    SERVE_SESSION.get_or_init(|| Mutex::new(None))
}

fn with_serve_session<R>(
    f: impl FnOnce(&mut Option<compiler::ServeCompileSession>) -> eyre::Result<R>,
) -> eyre::Result<R> {
    let lock = serve_session_lock();
    match lock.lock() {
        Ok(mut guard) => f(&mut guard),
        Err(poisoned) => {
            color_print::ceprintln!(
                "<y>Warning: serve session lock is poisoned; continuing with recovered state.</>"
            );
            let mut guard = poisoned.into_inner();
            f(&mut guard)
        }
    }
}

fn compile_serve_with_session(
    workspace: compiler::Workspace,
    dirty_paths: Option<&DirtySet>,
    outputs: compiler::CompileOutputs,
) -> eyre::Result<()> {
    with_serve_session(|slot| {
        let session = slot.get_or_insert_with(compiler::ServeCompileSession::default);
        match dirty_paths {
            Some(dirty_paths) if session.is_initialized() => {
                session.compile_incremental(workspace, dirty_paths, outputs)
            }
            _ => session.compile_full(workspace, outputs),
        }
    })
}

fn rewrite_serve_with_session(outputs: compiler::CompileOutputs) -> eyre::Result<()> {
    with_serve_session(|slot| {
        if let Some(session) = slot.as_mut() {
            return session.rewrite_all_from_memory(outputs);
        }

        // Fallback for edge cases where serve-session state was not initialized.
        let trees_dir = environment::trees_dir();
        let workspace = all_trees_source(&trees_dir)?;
        compiler::sync_typst_svg_assets(trees_dir.as_path(), None)?;
        let session = slot.get_or_insert_with(compiler::ServeCompileSession::default);
        session.compile_full(workspace, outputs)
    })
}

fn clear_serve_session() {
    let _ = with_serve_session(|slot| {
        *slot = None;
        Ok(())
    });
}

fn export_static_files() -> eyre::Result<()> {
    if !environment::inline_css() {
        sync_css_file(html_flake::html_main_style(), "main.css")?;
    }

    if !environment::inline_script() {
        sync_script_file(html_flake::html_main_script(), "main.js")?;
    }

    Ok(())
}

fn sync_css_file(css_content: &str, name: &str) -> eyre::Result<()> {
    let path = output_path(name);
    let path = Utf8Path::new(&path);
    sync_text_output(path, css_content, "CSS file")
}

fn sync_script_file(script_content: &str, name: &str) -> eyre::Result<()> {
    let path = output_path(name);
    let path = Utf8Path::new(&path);
    sync_text_output(path, script_content, "Script file")
}

fn sync_text_output(path: &Utf8Path, content: &str, label: &str) -> eyre::Result<()> {
    match fs::read_to_string(path.as_std_path()) {
        Ok(existing) if existing == content => return Ok(()),
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(err).wrap_err_with(|| eyre!("failed to read {} from \"{}\"", label, path));
        }
    }

    write_text_atomically(path, content, label)
}

fn write_text_atomically(path: &Utf8Path, content: &str, label: &str) -> eyre::Result<()> {
    environment::create_parent_dirs(path);
    let parent = path.parent().ok_or_else(|| {
        eyre!(
            "failed to resolve parent directory for {} `{}`",
            label,
            path
        )
    })?;
    let filename = path
        .file_name()
        .ok_or_else(|| eyre!("failed to resolve filename for {} `{}`", label, path))?;
    let temp_filename = format!(
        "{filename}.tmp.{}.{}",
        std::process::id(),
        next_atomic_write_stamp()
    );
    let temp_path = parent.join(temp_filename);

    let write_result = (|| -> eyre::Result<()> {
        let mut file = fs::File::create(temp_path.as_std_path())
            .wrap_err_with(|| eyre!("failed to create temp {} `{}`", label, temp_path))?;
        file.write_all(content.as_bytes())
            .wrap_err_with(|| eyre!("failed to write temp {} `{}`", label, temp_path))?;
        file.sync_all()
            .wrap_err_with(|| eyre!("failed to sync temp {} `{}`", label, temp_path))?;
        Ok(())
    })();

    if let Err(err) = write_result {
        let _ = fs::remove_file(temp_path.as_std_path());
        return Err(err);
    }

    fs::rename(temp_path.as_std_path(), path.as_std_path()).wrap_err_with(|| {
        eyre!(
            "failed to atomically replace {} `{}` from `{}`",
            label,
            path,
            temp_path
        )
    })?;

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
    next_atomic_write_stamp()
}

fn next_atomic_write_stamp() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = ATOMIC_WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{nanos}-{sequence}")
}

fn write_reload_marker_atomically(marker_path: &Utf8Path, stamp: &str) -> eyre::Result<()> {
    sync_text_output(marker_path, stamp, "reload marker")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_reload_marker_stamp_is_unique() {
        let a = next_reload_marker_stamp();
        let b = next_reload_marker_stamp();
        assert_ne!(a, b);
    }

    #[test]
    fn test_write_reload_marker_atomically_overwrites() {
        let base = crate::test_io::case_dir("reload-marker");
        let marker = base.join("serve/kodama.reload");

        write_reload_marker_atomically(marker.as_path(), "v1").unwrap();
        let first = fs::read_to_string(marker.as_std_path()).unwrap();
        assert_eq!(first, "v1");

        write_reload_marker_atomically(marker.as_path(), "v2").unwrap();
        let second = fs::read_to_string(marker.as_std_path()).unwrap();
        assert_eq!(second, "v2");

        let _ = fs::remove_dir_all(base.as_std_path());
    }

    #[test]
    fn test_sync_css_file_overwrites_when_content_changes() {
        let base = crate::test_io::case_dir("css-sync");
        let css_path = base.join("build/main.css");

        sync_text_output(css_path.as_path(), "body{color:black;}", "CSS file").unwrap();
        let first = fs::read_to_string(css_path.as_std_path()).unwrap();
        assert_eq!(first, "body{color:black;}");

        sync_text_output(css_path.as_path(), "body{color:white;}", "CSS file").unwrap();
        let second = fs::read_to_string(css_path.as_std_path()).unwrap();
        assert_eq!(second, "body{color:white;}");

        let _ = fs::remove_dir_all(base.as_std_path());
    }

    #[test]
    fn test_sync_script_file_overwrites_when_content_changes() {
        let base = crate::test_io::case_dir("script-sync");
        let script_path = base.join("build/main.js");

        sync_text_output(script_path.as_path(), "console.log(1);", "Script file").unwrap();
        let first = fs::read_to_string(script_path.as_std_path()).unwrap();
        assert_eq!(first, "console.log(1);");

        sync_text_output(script_path.as_path(), "console.log(2);", "Script file").unwrap();
        let second = fs::read_to_string(script_path.as_std_path()).unwrap();
        assert_eq!(second, "console.log(2);");

        let _ = fs::remove_dir_all(base.as_std_path());
    }
}
