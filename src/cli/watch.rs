// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use std::{io::Write, path::Path};

use notify::{event::ModifyKind, Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::config::{self, CompileConfig, FooterMode, OutputDir};

#[derive(clap::Args)]
pub struct WatchCommand {
    /// Path to output directory
    #[arg(short, long, default_value_t = OutputDir::default().0)]
    output: String,

    /// Configures watched files.
    #[arg(long)]
    dirs: Vec<String>,

    /// Configures the build script path.
    #[arg(short, long, default_value_t = ("./build.sh").to_string())]
    script: String,
}

pub fn watch(command: &WatchCommand) -> eyre::Result<()> {
    let output = &command.output;
    let dirs = &command.dirs;
    let script = &command.script;

    let _ = config::CONFIG.set(CompileConfig::new(
        config::RootDir::default(),
        config::TreesDir::default(), 
        config::OutputDir(output.to_string()),
        config::AssetsDir::default(),
        config::BaseUrl::default(),
        false,
        false,
        FooterMode::default(),
        false,
        None,
    ));

    if let Err(error) = watch_paths(dirs, script) {
        eprintln!("Error: {error:?}");
    }
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
    let output_dir = config::output_dir();
    let output_dir_name = output_dir.file_name().unwrap();

    for res in rx {
        match res {
            Ok(event) => {
                if !matches!(event.kind, EventKind::Modify(ModifyKind::Data(_))) {
                    // Ignore non-modify events
                    continue;
                }

                for path in event.paths {
                    if path.components().any(|c| c.as_os_str() == output_dir_name) {
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
            }
            Err(error) => eprintln!("Error: {error:?}"),
        }
    }

    Ok(())
}
