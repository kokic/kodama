// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{io::Write, sync::OnceLock};

use crate::{
    cli::build::{build_with_dirty, serve_rewrite_from_memory, BuildOptions},
    cli::output::OutputControlArgs,
    compiler::{CompileOutputs, DirtySet},
    config,
    environment::{self, BuildMode},
};

mod process;
mod watch;

use process::spawn_serve_process;
use watch::{
    analyze_watch_changes, compose_watched_paths, format_watch_change_stats,
    should_restart_for_config_change, watch_paths,
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

    /// Print watch dirty-path stats for each debounce batch.
    #[arg(short, long, default_value_t = false)]
    watch_stats: bool,

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

/// This function invoked the [`config::init_environment`] function to initialize the environment]
pub fn serve(command: &ServeCommand) -> eyre::Result<()> {
    _ = LIVE_RELOAD.set(!command.disable_reload);
    let outputs = compile_outputs(command);
    let watch_stats = command.watch_stats;
    let build_options = BuildOptions {
        verbose: command.verbose,
        verbose_skip: command.verbose_skip,
        no_cache: false,
        outputs,
    };

    let serve_build = |dirty_paths: Option<&DirtySet>| -> eyre::Result<()> {
        build_with_dirty(
            &command.config,
            BuildMode::Serve,
            build_options,
            dirty_paths,
        )?;
        Ok(())
    };

    let serve_rewrite = || -> eyre::Result<()> {
        serve_rewrite_from_memory(&command.config, build_options)?;
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
        let analysis = analyze_watch_changes(
            changed_paths,
            trees_dir.as_path(),
            trees_dir_canonical.as_path(),
            assets_dir.as_path(),
            assets_dir_canonical.as_path(),
        );
        if watch_stats {
            color_print::ceprintln!("<dim>{}</>", format_watch_change_stats(analysis.stats));
        }

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
        } else if !analysis.stats.has_effective_changes() {
            color_print::ceprintln!(
                "<dim>[watch] Skip rebuild: no effective changes after filtering.</>"
            );
        } else if analysis.stats.global_paths > 0 {
            // Non-tree changes (theme/import/html snippets) may affect all pages globally.
            // For tree changes in the same batch, update in-memory shallows first.
            if !analysis.dirty_paths.is_empty() {
                serve_build(Some(&analysis.dirty_paths))?;
            }
            // Then rewrite all pages from in-memory compile session state.
            serve_rewrite()?;
        } else {
            // Serve mode uses watcher-driven dirty set to avoid full hash scans on every rebuild.
            serve_build(Some(&analysis.dirty_paths))?;
        }
        Ok(())
    })?;

    // After watching process is done, kill the miniserve process.
    let _ = serve.kill();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_outputs_default_to_disabled_in_serve() {
        let command = ServeCommand {
            config: config::DEFAULT_CONFIG_PATH.into(),
            verbose: false,
            verbose_skip: false,
            disable_reload: false,
            watch_stats: false,
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
            watch_stats: false,
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
            watch_stats: false,
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
}
