// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

mod assets_sync;
mod cli;
mod compiler;
mod config;
mod config_toml;
mod entry;
mod html_flake;
mod html_macro;
mod process;
mod recorder;
mod slug;
mod typst_cli;

use clap::Parser;

use crate::cli::{build::BuildCommand, remove::RemoveCommand, watch::WatchCommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Create a new section.
    #[command(visible_alias = "new")]
    New(crate::cli::new::NewCommand),

    /// Compile current workspace dir to HTMLs.
    ///
    /// This is a config dependent command.
    #[command(visible_alias = "b")]
    Build(BuildCommand),

    /// Watch files and run build script on changes.
    ///
    /// This is a config dependent command.
    #[command(visible_alias = "w")]
    Watch(WatchCommand),

    /// Remove associated files (hash, entry & HTML) for the given section paths.
    ///
    /// This is a config dependent command.
    #[command(visible_alias = "rm")]
    Remove(RemoveCommand),
    // TODO: Move.
    //
    // We are temporarily putting this feature on hold because we have not yet exported the dependency information for the section.
}

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Command::New(command) => crate::cli::new::new(command)?,
        Command::Build(command) => crate::cli::build::compile(command)?,
        Command::Remove(command) => crate::cli::remove::remove(command)?,
        Command::Watch(command) => crate::cli::watch::watch(command)?,
    };
    Ok(())
}
