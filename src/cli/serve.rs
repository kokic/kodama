// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use std::{io::Write, path::Path};

use notify::{event::ModifyKind, Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{cli::build::build_with, config::BuildMode, config_toml};

#[derive(clap::Args)]
pub struct ServeCommand {
    /// Path to the configuration file (e.g., "kodama.toml").
    #[arg(short, long, default_value_t = config_toml::DEFAULT_CONFIG_PATH.into())]
    config: String,
}

/// This function invoked the [`config_toml::apply_config`] function to apply the configuration.
pub fn serve(command: &ServeCommand) -> eyre::Result<()> {
    build_with(command.config.clone(), BuildMode::Serve)?;

    // miniserve <publish_dir> --index index.html --pretty-urls
    std::process::Command::new("miniserve")
        .arg(crate::config::output_dir())
        .arg("--index")
        .arg("index.html")
        .arg("--pretty-urls")
        .spawn()?;

    Ok(())
}

/// from: https://github.com/notify-rs/notify/blob/main/examples/monitor_raw.rs#L18
fn watch_paths<P: AsRef<Path>>(watched_paths: &Vec<P>, script_path: &str) -> notify::Result<()> {
    print!("\x1B[2J\x1B[H");
    std::io::stdout().flush()?;

    let (tx, rx) = std::sync::mpsc::channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    // All files and directories at that path and
    // below will be monitored for changes.

    print!("Watching: ");
    for watched_path in watched_paths {
        let watched_path = watched_path.as_ref();
        watcher.watch(watched_path, RecursiveMode::Recursive)?;
        print!("\"{}\"  ", watched_path.to_string_lossy());
    }
    println!("\n\nPress Ctrl+C to stop watching.\n");

    let row = watched_paths.len() + 1;
    for res in rx {
        match res {
            Ok(event) => {
                if !matches!(event.kind, EventKind::Modify(ModifyKind::Data(_))) {
                    // Ignore non-modify events
                    continue;
                }

                for path in event.paths {
                    print!("\x1B[{};0H\x1B[2K", row);
                    print!("Change: {path:?}");
                    std::io::stdout().flush()?;

                    let output = std::process::Command::new(script_path)
                        .stdout(std::process::Stdio::piped())
                        .output()
                        .expect("Build command failed to start");

                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        eprintln!("{}", stderr);
                    }
                }
            }
            Err(error) => eprintln!("Error: {error:?}"),
        }
    }

    Ok(())
}
