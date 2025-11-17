// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::io::Write;

use camino::{Utf8Path, Utf8PathBuf};
use colored::Colorize;
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
}

/// This function invoked the [`config::init_environment`] function to initialize the environment]
pub fn serve(command: &ServeCommand) -> eyre::Result<()> {
    let serve_build = || -> eyre::Result<()> {
        build_with(&command.config, BuildMode::Serve)?;
        Ok(())
    };

    serve_build()?;

    print!("\x1B[2J\x1B[H");
    std::io::stdout().flush()?;

    let mut serve = parse_command(
        &environment::serve_command(),
        crate::environment::output_dir(),
    )?
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped())
    .spawn()?;

    let serve_stdout = serve.stdout.take().unwrap();
    let serve_stderr = serve.stderr.take().unwrap();

    std::thread::spawn(move || {
        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(serve_stdout);
        for line in reader.lines() {
            println!("[serve] {}", line.unwrap());
        }
    });

    std::thread::spawn(move || {
        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(serve_stderr);
        for line in reader.lines() {
            let message = format!("[serve] Error: {}", line.unwrap()).red();
            eprintln!("{message}");
        }
    });

    watch_paths(
        &vec![
            crate::environment::trees_dir(),
            crate::environment::assets_dir(),
        ],
        |_| serve_build(),
    )?;

    // After watching process is done, kill the miniserve process.
    let _ = serve.kill();

    Ok(())
}

fn parse_command(command: &[String], output: Utf8PathBuf) -> eyre::Result<std::process::Command> {
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

/// from: https://github.com/notify-rs/notify/blob/main/examples/monitor_raw.rs#L18
fn watch_paths<P: AsRef<Utf8Path>, F>(watched_paths: &Vec<P>, action: F) -> eyre::Result<()>
where
    F: Fn(&Utf8Path) -> eyre::Result<()>,
{
    let (tx, rx) = std::sync::mpsc::channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    // All files and directories at that path and
    // below will be monitored for changes.

    print!("[watch] ");
    for watched_path in watched_paths {
        let watched_path = watched_path.as_ref();
        if !watched_path.exists() {
            let message = format!(
                "[watch] Warning: Path \"{}\" does not exist, skipping.",
                watched_path
            )
            .yellow();
            eprintln!("{message}");
            continue;
        }

        watcher.watch(watched_path.as_std_path(), RecursiveMode::Recursive)?;
        print!("\"{}\"  ", watched_path);
    }
    println!("\n\nPress Ctrl+C to stop watching.\n");

    for res in rx {
        match res {
            Ok(event) => {
                // Generally, we only need to listen for changes in file content `ModifyKind::Data(_)`,
                // but since notify-rs always only gets `Modify(Any)` on Windows,
                // we expand the listening scope here.
                if let EventKind::Modify(_) = event.kind {
                    for path in event.paths {
                        println!("[watch] Change: {path:?}");
                        std::io::stdout().flush()?;
                        if let Ok(p) = path.as_path().try_into() {
                            action(p)?;
                        }
                    }
                }
            }
            Err(error) => {
                let message = format!("[watch] Error: {error:?}").red();
                eprintln!("{message}");
            }
        }
    }

    Ok(())
}
