// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{io::Write, sync::OnceLock};

use camino::{Utf8Path, Utf8PathBuf};
use eyre::eyre;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{
    cli::build::build_with,
    config,
    environment::{self, BuildMode},
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
}

static LIVE_RELOAD: OnceLock<bool> = OnceLock::new();

pub fn live_reload() -> &'static bool {
    LIVE_RELOAD.get().unwrap_or(&true)
}

/// This function invoked the [`config::init_environment`] function to initialize the environment]
pub fn serve(command: &ServeCommand) -> eyre::Result<()> {
    _ = LIVE_RELOAD.set(!command.disable_reload);

    let serve_build = || -> eyre::Result<()> {
        build_with(&command.config, BuildMode::Serve, command.verbose, command.verbose_skip, false)?;
        Ok(())
    };

    serve_build()?;
    let config_file = environment::config_file();
    let config_file_canonical = config_file
        .canonicalize_utf8()
        .unwrap_or_else(|_| config_file.clone());

    print!("\x1B[2J\x1B[H");
    std::io::stdout().flush()?;

    let mut serve = spawn_serve_process()?;

    let mut watched_paths = vec![
        crate::environment::trees_dir(),
        crate::environment::assets_dir(),
        config_file.clone(),
        crate::environment::root_dir().join("import-meta.html"),
        crate::environment::root_dir().join("import-style.html"),
        crate::environment::root_dir().join("import-font.html"),
        crate::environment::root_dir().join("import-math.html"),
    ];
    watched_paths.extend(crate::environment::theme_paths());
    watch_paths(&watched_paths, |changed_path| {
        serve_build()?;
        let changed_canonical = changed_path
            .canonicalize_utf8()
            .unwrap_or_else(|_| changed_path.to_owned());
        if changed_path == config_file.as_path() || changed_canonical == config_file_canonical {
            color_print::ceprintln!("<y>[watch] Config changed. Restarting serve process.</>");
            let _ = serve.kill();
            let _ = serve.wait();
            serve = spawn_serve_process()?;
        }
        Ok(())
    })?;

    // After watching process is done, kill the miniserve process.
    let _ = serve.kill();

    Ok(())
}

fn parse_command(command: &[String], output: Utf8PathBuf) -> eyre::Result<std::process::Command> {
    if command.is_empty() {
        return Err(eyre!("invalid `serve.command`: command list cannot be empty"));
    }

    let mut serve = std::process::Command::new(&command[0]);
    for arg in &command[1..] {
        if arg == "<output>" {
            serve.arg(&output);
            continue;
        }
        serve.arg(arg);
    }
    Ok(serve)
}

fn spawn_serve_process() -> eyre::Result<std::process::Child> {
    let command = environment::serve_command();
    let mut serve = parse_command(&command, crate::environment::output_dir())?
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(serve_stdout) = serve.stdout.take() {
        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(serve_stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => println!("[serve] {line}"),
                    Err(err) => {
                        color_print::ceprintln!("<r>[serve] stdout read error: {err}</>");
                        break;
                    }
                }
            }
        });
    }

    if let Some(serve_stderr) = serve.stderr.take() {
        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(serve_stderr);
            for line in reader.lines() {
                match line {
                    Ok(line) => color_print::ceprintln!("<r>[serve] Error: {line}</>"),
                    Err(err) => {
                        color_print::ceprintln!("<r>[serve] stderr read error: {err}</>");
                        break;
                    }
                }
            }
        });
    }

    Ok(serve)
}

/// from: https://github.com/notify-rs/notify/blob/main/examples/monitor_raw.rs#L18
fn watch_paths<P: AsRef<Utf8Path>, F>(watched_paths: &[P], mut action: F) -> eyre::Result<()>
where
    F: FnMut(&Utf8Path) -> eyre::Result<()>,
{
    let (tx, rx) = std::sync::mpsc::channel();
    let debounce = std::time::Duration::from_millis(250);
    let mut last_run = std::time::Instant::now() - debounce;

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    // All files and directories at that path and
    // below will be monitored for changes.

    print!("[watch] ");
    for watched_path in watched_paths {
        let watched_path = watched_path.as_ref();
        if !watched_path.exists() {
            color_print::ceprintln!(
                "<y>[watch] Warning: Path \"{}\" does not exist, skipping.</>",
                watched_path
            );
            continue;
        }

        let mode = if watched_path.is_file() {
            RecursiveMode::NonRecursive
        } else {
            RecursiveMode::Recursive
        };
        watcher.watch(watched_path.as_std_path(), mode)?;
        print!("\"{}\"  ", watched_path);
    }
    println!("\n\nPress Ctrl+C to stop watching.\n");

    for res in rx {
        match res {
            Ok(event) => {
                // Generally, we only need to listen for changes in file content `ModifyKind::Data(_)`,
                // but since notify-rs always only gets `Modify(Any)` on Windows,
                // we expand the listening scope here.
                if matches!(
                    event.kind,
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) | EventKind::Any
                ) {
                    let now = std::time::Instant::now();
                    if now.duration_since(last_run) < debounce {
                        continue;
                    }

                    if let Some(path) = event.paths.iter().find_map(|path| path.as_path().try_into().ok()) {
                        println!("[watch] Change: {path:?}");
                        std::io::stdout().flush()?;
                        if let Err(err) = action(path) {
                            color_print::ceprintln!("<r>[watch] Rebuild failed: {}</>", err);
                        }
                        last_run = now;
                    }
                }
            }
            Err(error) => {
                color_print::ceprintln!("<r>[watch] Error: {error:?}</>");
            }
        }
    }

    Ok(())
}
