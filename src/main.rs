// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

mod assets_sync;
mod compiler;
mod config;
mod entry;
mod html_flake;
mod html_macro;
mod process;
mod recorder;
mod slug;
mod typst_cli;

use config::{join_path, output_path, CompileConfig, FooterMode};

use std::{fs, io::Write, path::Path};

use clap::Parser;
use eyre::{eyre, WrapErr};

use notify::{event::ModifyKind, Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Compile current workspace dir to HTMLs.
    #[command(visible_alias = "c")]
    Compile(CompileCommand),

    /// Clean build files (.cache & publish).
    Clean(CleanCommand),

    /// Watch files and run build script on changes.
    #[command(visible_alias = "w")]
    Watch(WatchCommand),
}

#[derive(clap::Args)]
struct CompileCommand {
    /// Base URL or publish URL (e.g. https://www.example.com/)
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG.base_url.into())]
    base: String,

    /// Path to output directory
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG.output_dir.into())]
    output: String,

    /// Path to assets directory relative to the output directory
    #[arg(long, default_value_t = config::DEFAULT_CONFIG.assets_dir.into())]
    assets: String,

    /// Configures the project root (for absolute paths)
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG.root_dir.into())]
    root: String,

    /// Disable pretty urls (`/page` to `/page.html`)
    #[arg(short, long)]
    disable_pretty_urls: bool,

    /// Hide parents part in slug (e.g. `tutorials/install` to `install`)
    #[arg(short, long)]
    short_slug: bool,

    /// Specify the inline mode for the footer sections
    #[arg(short, long, default_value_t = FooterMode::Link)]
    footer_mode: FooterMode,

    /// Disable exporting the `*.css` file to the output directory
    #[arg(long)]
    disable_export_css: bool,

    /// Display URL redirect links prepared for the editor (e.g. `vscode://file:`)
    #[arg(long)]
    edit: Option<String>,
}

#[derive(clap::Args)]
struct CleanCommand {
    /// Path to output dir.
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG.output_dir.into())]
    output: String,

    /// Configures the project root (for absolute paths)
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG.root_dir.into())]
    root: String,

    /// Clean markdown hash files.
    #[arg(short, long)]
    markdown: bool,

    /// Clean typ hash files.
    #[arg(long)]
    typ: bool,

    /// Clean typst hash files.
    #[arg(long)]
    typst: bool,

    /// Clean html hash files.
    #[arg(long)]
    html: bool,
}

#[derive(clap::Args)]
struct WatchCommand {
    /// Configures watched files.
    #[arg(long)]
    dirs: Vec<String>,

    /// Configures the build script path. 
    #[arg(short, long, default_value_t = ("./build.sh").to_string())]
    script: String,
}

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Command::Compile(compile_command) => {
            let root = &compile_command.root;
            let output = &compile_command.output;

            config::mutex_set(
                &config::CONFIG,
                CompileConfig::new(
                    root.to_string(),
                    output.to_string(),
                    compile_command.assets.to_string(),
                    compile_command.base.to_string(),
                    compile_command.disable_pretty_urls,
                    compile_command.short_slug,
                    compile_command.footer_mode.clone(),
                    compile_command.disable_export_css,
                    compile_command.edit.clone(),
                ),
            );

            match &compile_command.edit {
                Some(s) => println!("[{}] EDIT MODE IS ENABLE. Please note that your disk file path information will be included in the pages!", s),
                None => (),
            }

            if !compile_command.disable_export_css {
                export_css_files().wrap_err("failed to export CSS")?;
            }

            compiler::compile_all(root)
                .wrap_err_with(|| eyre!("failed to compile project `{root}`"))?;

            sync_assets_dir()?;
        }
        Command::Clean(clean_command) => {
            config::mutex_set(
                &config::CONFIG,
                CompileConfig::new(
                    clean_command.root.to_string(),
                    clean_command.output.to_string(),
                    config::DEFAULT_CONFIG.assets_dir.into(),
                    config::DEFAULT_CONFIG.base_url.into(),
                    false,
                    config::DEFAULT_CONFIG.short_slug,
                    FooterMode::Link,
                    true,
                    None,
                ),
            );

            let cache_dir = config::get_cache_dir();

            let path_ends_with =
                |suffix: &'static str| move |p: &Path| p.to_string_lossy().ends_with(suffix);

            clean_command.markdown.then(|| {
                let _ = config::delete_all_with(&cache_dir, &path_ends_with(".md.hash"));
            });

            clean_command.typ.then(|| {
                let _ = config::delete_all_with(&cache_dir, &path_ends_with(".typ.hash"));
            });

            clean_command.typst.then(|| {
                let _ = config::delete_all_with(&cache_dir, &path_ends_with(".typst.hash"));
            });

            clean_command.html.then(|| {
                let _ = config::delete_all_with(&cache_dir, &path_ends_with(".html.hash"));
            });
        }
        Command::Watch(watch_command) => {
            if let Err(error) = watch(&watch_command.dirs, &watch_command.script) {
                eprintln!("Error: {error:?}");
            }
        }
    }
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
            .wrap_err_with(|| eyre!("failed to write CSS file to `{}`", path.display()))?;
    }
    Ok(())
}

fn sync_assets_dir() -> eyre::Result<bool> {
    let source = join_path(&config::root_dir(), "assets");
    let target = join_path(&config::output_dir(), "assets");
    assets_sync::sync_assets(source, target)?;
    Ok(true)
}

/// from: https://github.com/notify-rs/notify/blob/main/examples/monitor_raw.rs#L18
fn watch<P: AsRef<Path>>(watched_paths: &Vec<P>, script_path: &str) -> notify::Result<()> {
    print!("\x1B[2J\x1B[H");
    std::io::stdout().flush()?;

    let (tx, rx) = std::sync::mpsc::channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    // All files and directories at that path and
    // below will be monitored for changes.
    for watched_path in watched_paths {
        let watched_path = watched_path.as_ref();
        watcher.watch(watched_path, RecursiveMode::Recursive)?;

        println!("Watching: {}", watched_path.to_string_lossy());
    }

    let row = watched_paths.len() + 1;

    for res in rx {
        match res {
            Ok(event) => {
                if !matches!(event.kind, EventKind::Modify(ModifyKind::Data(_))) {
                    // Ignore non-modify events
                    continue;
                }

                for path in event.paths {
                    let path_lossy = path.to_string_lossy();
                    if !path_lossy.contains("/publish/") && !path.ends_with("publish") {
                        print!("\x1B[{};0H\x1B[2K", row);
                        print!("Change: {path:?}");
                        std::io::stdout().flush()?;

                        let output = std::process::Command::new(script_path)
                            .stdout(std::process::Stdio::piped())
                            .output()
                            .expect("build command failed to start");

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
