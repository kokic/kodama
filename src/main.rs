// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

mod assets_sync;
mod cli;
mod compiler;
mod config;
mod entry;
mod environment;
mod html_flake;
mod html_macro;
mod ordered_map;
mod path_utils;
mod process;
mod recorder;
mod slug;
mod typst_cli;

use clap::Parser;

use crate::cli::{
    build::BuildCommand,
    init::InitCommand,
    new::{NewCommand, NewCommandCli},
    serve::ServeCommand, snip::SnipCommand,
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Create a new kodama site / config / post.
    #[command(visible_alias = "n")]
    New(NewCommandCli),

    /// Create a new kodama site in an existing directory.
    #[command(visible_alias = "i")]
    Init(InitCommand),

    /// Compile current workspace dir to HTMLs.
    #[command(visible_alias = "b")]
    Build(BuildCommand),

    /// Serve a forest at http://localhost:<port>, and rebuilds it on changes.
    ///
    /// Server by default depends on the miniserve program in the user's environment.
    #[command(visible_alias = "s")]
    Serve(ServeCommand),

    /// Generate VSCode style section link snippets file.
    #[command()]
    Snip(SnipCommand),
}

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();
    match &cli.command {
        Command::New(NewCommandCli { command }) => match command {
            NewCommand::Site(command) => crate::cli::new::new_site(command)?,
            NewCommand::Post(command) => crate::cli::new::new_section(command)?,
            NewCommand::Config(command) => crate::cli::new::new_config(command)?,
        },
        Command::Init(command) => crate::cli::init::init(command)?,
        Command::Serve(command) => crate::cli::serve::serve(command)?,
        Command::Build(command) => crate::cli::build::build(command)?,
        Command::Snip(command) => crate::cli::snip::snip(command)?,
    };
    Ok(())
}
