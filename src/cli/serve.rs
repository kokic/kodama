// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{io::Write, sync::OnceLock};

use camino::{Utf8Path, Utf8PathBuf};

use crate::{
    cli::build::{build_with_dirty, BuildOptions},
    cli::output::OutputControlArgs,
    compiler::{CompileOutputs, DirtySet},
    config,
    environment::{self, BuildMode},
};

mod process;
mod watch;

use process::spawn_serve_process;
use watch::{
    compose_dirty_paths, compose_watched_paths, should_restart_for_config_change, watch_paths,
};

#[derive(clap::Args)]
pub struct ServeCommand {
    /// Path to the configuration file (e.g., "Kodama.toml").
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG_PATH.into())]
    config: String,

    /// Enable verbose output.
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Enable verbose skip output.
    #[arg(long, default_value_t = false)]
    verbose_skip: bool,

    /// Disable live reload.
    #[arg(short, long, default_value_t = false)]
    disable_reload: bool,

    #[command(flatten)]
    output: OutputControlArgs,
}

static LIVE_RELOAD: OnceLock<bool> = OnceLock::new();

pub fn live_reload() -> &'static bool {
    LIVE_RELOAD.get().unwrap_or(&true)
}

fn compile_outputs(command: &ServeCommand) -> CompileOutputs {
    command.output.resolve(CompileOutputs {
        indexes: false,
        graph: false,
    })
}

fn canonicalize_or_self(path: &Utf8Path) -> Utf8PathBuf {
    path.canonicalize_utf8().unwrap_or_else(|_| path.to_owned())
}

fn is_path_under_dir(path: &Utf8Path, dir: &Utf8Path, dir_canonical: &Utf8Path) -> bool {
    path.starts_with(dir) || path.starts_with(dir_canonical) || {
        let canonical = canonicalize_or_self(path);
        canonical.starts_with(dir) || canonical.starts_with(dir_canonical)
    }
}

/// This function invoked the [`config::init_environment`] function to initialize the environment]
pub fn serve(command: &ServeCommand) -> eyre::Result<()> {
    _ = LIVE_RELOAD.set(!command.disable_reload);
    let outputs = compile_outputs(command);

    let serve_build = |dirty_paths: Option<&DirtySet>| -> eyre::Result<()> {
        build_with_dirty(
            &command.config,
            BuildMode::Serve,
            BuildOptions {
                verbose: command.verbose,
                verbose_skip: command.verbose_skip,
                no_cache: false,
                outputs,
            },
            dirty_paths,
        )?;
        Ok(())
    };

    serve_build(None)?;
    let config_file = environment::config_file();
    let config_file_canonical = config_file
        .canonicalize_utf8()
        .unwrap_or_else(|_| config_file.clone());

    print!("\x1B[2J\x1B[H");
    std::io::stdout().flush()?;

    let mut serve = spawn_serve_process()?;

    let root_dir = crate::environment::root_dir();
    let trees_dir = crate::environment::trees_dir();
    let assets_dir = crate::environment::assets_dir();
    let assets_dir_canonical = assets_dir
        .canonicalize_utf8()
        .unwrap_or_else(|_| assets_dir.clone());
    let trees_dir_canonical = trees_dir
        .canonicalize_utf8()
        .unwrap_or_else(|_| trees_dir.clone());
    let watched_paths = compose_watched_paths(
        root_dir.as_path(),
        trees_dir.clone(),
        assets_dir.clone(),
        config_file.clone(),
        crate::environment::theme_paths(),
    );
    watch_paths(&watched_paths, assets_dir.as_path(), |changed_paths| {
        let dirty_paths = compose_dirty_paths(
            changed_paths,
            trees_dir.as_path(),
            trees_dir_canonical.as_path(),
        );
        let should_restart = changed_paths.iter().any(|changed_path| {
            should_restart_for_config_change(
                changed_path.as_path(),
                config_file.as_path(),
                config_file_canonical.as_path(),
            )
        });

        if should_restart {
            // Config changes can alter compiler behavior globally; keep full-hash baseline here.
            serve_build(None)?;
            color_print::ceprintln!("<y>[watch] Config changed. Restarting serve process.</>");
            let _ = serve.kill();
            let _ = serve.wait();
            serve = spawn_serve_process()?;
        } else if changed_paths.iter().any(|changed_path| {
            !is_path_under_dir(
                changed_path.as_path(),
                trees_dir.as_path(),
                trees_dir_canonical.as_path(),
            ) && !is_path_under_dir(
                changed_path.as_path(),
                assets_dir.as_path(),
                assets_dir_canonical.as_path(),
            )
        }) {
            // Non-tree changes (theme/import/html snippets) may affect all pages globally.
            serve_build(None)?;
        } else {
            // Serve mode uses watcher-driven dirty set to avoid full hash scans on every rebuild.
            serve_build(Some(&dirty_paths))?;
        }
        Ok(())
    })?;

    // After watching process is done, kill the miniserve process.
    let _ = serve.kill();

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use camino::Utf8PathBuf;

    use super::*;

    fn case_dir(name: &str) -> Utf8PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("kodama-serve-{name}-{}", fastrand::u64(..)));
        Utf8PathBuf::from_path_buf(path).expect("temp path should be valid utf8")
    }

    #[test]
    fn test_compile_outputs_default_to_disabled_in_serve() {
        let command = ServeCommand {
            config: config::DEFAULT_CONFIG_PATH.into(),
            verbose: false,
            verbose_skip: false,
            disable_reload: false,
            output: OutputControlArgs::default(),
        };
        let outputs = compile_outputs(&command);
        assert!(!outputs.indexes);
        assert!(!outputs.graph);
    }

    #[test]
    fn test_compile_outputs_can_be_enabled_in_serve() {
        let command = ServeCommand {
            config: config::DEFAULT_CONFIG_PATH.into(),
            verbose: false,
            verbose_skip: false,
            disable_reload: false,
            output: OutputControlArgs {
                indexes: true,
                no_indexes: false,
                graph: true,
                no_graph: false,
            },
        };
        let outputs = compile_outputs(&command);
        assert!(outputs.indexes);
        assert!(outputs.graph);
    }

    #[test]
    fn test_compile_outputs_can_be_disabled_with_compat_flags_in_serve() {
        let command = ServeCommand {
            config: config::DEFAULT_CONFIG_PATH.into(),
            verbose: false,
            verbose_skip: false,
            disable_reload: false,
            output: OutputControlArgs {
                indexes: false,
                no_indexes: true,
                graph: false,
                no_graph: true,
            },
        };
        let outputs = compile_outputs(&command);
        assert!(!outputs.indexes);
        assert!(!outputs.graph);
    }

    #[test]
    fn test_is_path_under_dir_matches_non_canonical_and_canonical_paths() {
        let root = case_dir("path-under-dir");
        let trees = root.join("trees");
        fs::create_dir_all(trees.as_std_path()).unwrap();

        let changed = root.join("trees/sub/../index.md");
        let trees_canonical = trees.canonicalize_utf8().unwrap();
        assert!(is_path_under_dir(
            changed.as_path(),
            trees.as_path(),
            trees_canonical.as_path()
        ));

        let outside = root.join("themes/theme.html");
        assert!(!is_path_under_dir(
            outside.as_path(),
            trees.as_path(),
            trees_canonical.as_path()
        ));

        let _ = fs::remove_dir_all(root);
    }
}
