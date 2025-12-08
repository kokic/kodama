// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use crate::cli::new::add_project_files;
use camino::Utf8PathBuf;

#[derive(clap::Args)]
pub struct InitCommand {
    /// Path to the new site.
    #[arg(default_value = "./")]
    pub path: Utf8PathBuf,

    /// Do not create Typst library files.
    #[arg(alias = "nt", long, default_value_t = false)]
    pub no_typst: bool,
}

pub fn init(command: &InitCommand) -> eyre::Result<()> {
    let site_path = &command.path;
    if !site_path.exists() {
        return Err(eyre::eyre!("Does not exist: {}", site_path));
    }

    add_project_files(site_path, command.no_typst)?;
    Ok(())
}
