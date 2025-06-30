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

use config::{output_path, CompileConfig, FooterMode};

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

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
    /// 
    /// This is a config dependent command. 
    #[command(visible_alias = "c")]
    Compile(CompileCommand),

    /// Watch files and run build script on changes.
    #[command(visible_alias = "w")]
    Watch(WatchCommand),

    /// Remove associated files (hash, entry & HTML) for the given section paths.
    /// 
    /// This is a config dependent command. 
    #[command(visible_alias = "rm")]
    Remove {
        /// Section paths to remove.
        #[arg(required = true)]
        path: Vec<PathBuf>,

        /// Path to output directory
        #[arg(short, long, default_value_t = config::DEFAULT_CONFIG.output_dir.into())]
        output: String,
    },
    // TODO: Move.
    //
    // We are temporarily putting this feature on hold because we have not yet exported the dependency information for the section.
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
                export_css_files().wrap_err("Failed to export CSS")?;
            }

            compiler::compile_all(root)
                .wrap_err_with(|| eyre!("Failed to compile project `{root}`"))?;

            sync_assets_dir()?;
        }
        Command::Remove { path, output } => {
            config::mutex_set(
                &config::CONFIG,
                CompileConfig::new(
                    config::DEFAULT_CONFIG.root_dir.to_string(),
                    output.to_string(),
                    config::DEFAULT_CONFIG.assets_dir.to_string(),
                    config::DEFAULT_CONFIG.base_url.to_string(),
                    false,
                    config::DEFAULT_CONFIG.short_slug,
                    config::DEFAULT_CONFIG.footer_mode.clone(),
                    config::DEFAULT_CONFIG.disable_export_css,
                    None,
                ),
            );

            for section_path in path {
                remove_with_hint(section_path)?;
                remove_with_hint(config::hash_file_path(section_path))?;
                remove_with_hint(config::entry_file_path(section_path))?;
                remove_with_hint(config::output_html_path(section_path))?;
            }
        }
        Command::Watch(watch_command) => {
            if let Err(error) = watch(&watch_command.dirs, &watch_command.script) {
                eprintln!("Error: {error:?}");
            }
        }
    }
    Ok(())
}

fn remove_with_hint<P: AsRef<Path>>(path: P) -> eyre::Result<()> {
    let path = path.as_ref();
    if path.exists() {
        fs::remove_file(path)
            .wrap_err_with(|| eyre!("Failed to remove file `{}`", path.display()))?;
        println!("Removed: \"{}\"", path.display());
    } else {
        println!("File \"{}\" does not exist, skipping.", path.display());
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
            .wrap_err_with(|| eyre!("Failed to write CSS file to \"{}\"", path.display()))?;
    }
    Ok(())
}

fn sync_assets_dir() -> eyre::Result<bool> {
    let source = config::root_dir().join(config::assets_dir());
    let target = config::output_dir().join(config::assets_dir());
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
