// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use std::{
    fs,
    path::{Path, PathBuf},
};

use eyre::{eyre, WrapErr};

use crate::config::{self, CompileConfig, FooterMode};

#[derive(clap::Args)]
pub struct RemoveCommand {
    /// Path to section to remove.
    #[arg(required = true)]
    pub path: Vec<PathBuf>,

    /// Path to output directory.
    #[arg(short, long, default_value_t)]
    pub output: String,
}
/// Section paths to remove.

pub fn remove(command: &RemoveCommand) -> eyre::Result<()> {
    let path = &command.path;
    let output = &command.output;

    let _ = config::CONFIG.set(CompileConfig::new(
        config::RootDir::default(),
        config::OutputDir(output.to_string()),
        config::AssetsDir::default(),
        config::BaseUrl::default(),
        false,
        false,
        FooterMode::default(),
        false,
        None,
    ));

    for section_path in path {
        remove_with_hint(section_path)?;
        remove_with_hint(config::hash_file_path(section_path))?;
        remove_with_hint(config::entry_file_path(section_path))?;
        remove_with_hint(config::output_html_path(section_path))?;
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
