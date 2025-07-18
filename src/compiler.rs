// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Alias Qli (@AliasQli), Spore (@s-cerevisiae)

pub mod callback;
pub mod counter;
pub mod html_parser;
pub mod parser;
pub mod section;
pub mod state;
pub mod taxon;
pub mod typst;
pub mod writer;

use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

use eyre::{bail, eyre, Ok, WrapErr};
use parser::parse_markdown;
use section::{HTMLContent, ShallowSection};
use typst::parse_typst;
use walkdir::WalkDir;
use writer::Writer;

use crate::{
    config::{self, verify_and_file_hash},
    slug::{self, Ext, Slug},
};

pub fn compile(workspace: Workspace) -> eyre::Result<()> {
    let mut shallows = HashMap::new();

    for (&slug, &ext) in &workspace.slug_exts {
        let relative_path = format!("{}.{}", slug, ext);

        let is_modified = match config::is_serve() {
            true => verify_and_file_hash(&relative_path)
            .wrap_err_with(|| eyre!("failed to verify hash of `{relative_path}`"))?,
            false => true,
        };
        
        let entry_path = config::entry_file_path(&relative_path);
        let shallow = if !is_modified && entry_path.exists() {
            let entry_file = BufReader::new(File::open(&entry_path).wrap_err_with(|| {
                eyre!("failed to open entry file at `{}`", entry_path.display())
            })?);
            let shallow: ShallowSection =
                serde_json::from_reader(entry_file).wrap_err_with(|| {
                    eyre!(
                        "failed to deserialize entry file at `{}`",
                        entry_path.display()
                    )
                })?;
            shallow
        } else {
            let shallow = match ext {
                Ext::Markdown => parse_markdown(slug)
                    .wrap_err_with(|| eyre!("failed to parse markdown file `{slug}.{ext}`"))?,
                Ext::Typst => parse_typst(slug, config::typst_root_dir())
                    .wrap_err_with(|| eyre!("failed to parse typst file `{slug}.{ext}`"))?,
            };
            let serialized = serde_json::to_string(&shallow).unwrap();
            std::fs::write(&entry_path, serialized)
                .wrap_err_with(|| eyre!("failed to write entry to `{}`", entry_path.display()))?;

            shallow
        };

        shallows.insert(slug, shallow);
    }

    let state = state::compile_all(shallows)?;
    Writer::write_needed_slugs(workspace.slug_exts.into_iter().map(|x| x.0), &state);

    Ok(())
}

pub fn should_ignored_file(path: &Path) -> bool {
    let name = path.file_name().unwrap();
    name == "README.md"
}

pub fn should_ignored_dir(path: &Path) -> bool {
    let name = path.file_name().unwrap();
    name.to_str()
        .is_some_and(|s| s.starts_with('.') || s.starts_with('_'))
}

fn to_slug_ext(source_dir: &Path, p: &Path) -> Option<(Slug, Ext)> {
    let p = p.strip_prefix(source_dir).unwrap_or(p);
    let ext = p.extension()?.to_str()?.parse().ok()?;
    let slug = Slug::new(slug::pretty_path(&p.with_extension("")));
    Some((slug, ext))
}

/// Collect all source file paths in workspace dir.
///
/// It includes all `.md` and `.typ` files in the `trees_dir`.
pub fn all_trees_source(trees_dir: &Path) -> eyre::Result<Workspace> {
    let mut slug_exts = HashMap::new();

    let failed_to_read_dir = |dir: &Path| eyre!("failed to read directory `{}`", dir.display());
    let file_collide = |p: &Path, e: Ext| {
        eyre!(
            "`{}` collides with `{}`",
            p.display(),
            p.with_extension(e.to_string()).display(),
        )
    };

    let mut collect_files = |source_dir: &Path| {
        for entry in
            std::fs::read_dir(source_dir).wrap_err_with(|| failed_to_read_dir(source_dir))?
        {
            let path = entry
                .wrap_err_with(|| failed_to_read_dir(source_dir))?
                .path();
            if path.is_file() && !should_ignored_file(&path) {
                let Some((slug, ext)) = to_slug_ext(source_dir, &path) else {
                    continue;
                };
                if let Some(ext) = slug_exts.insert(slug, ext) {
                    bail!(file_collide(&path, ext));
                };
            } else if path.is_dir() && !should_ignored_dir(&path) {
                for entry in WalkDir::new(&path)
                    .follow_links(true)
                    .into_iter()
                    .filter_entry(|e| {
                        let path = e.path();
                        path.is_file() || !should_ignored_dir(path)
                    })
                {
                    let path = entry
                        .wrap_err_with(|| failed_to_read_dir(&path))?
                        .into_path();
                    if path.is_file() {
                        let Some((slug, ext)) = to_slug_ext(source_dir, &path) else {
                            continue;
                        };
                        if let Some(ext) = slug_exts.insert(slug, ext) {
                            bail!(file_collide(&path, ext));
                        }
                    }
                }
            }
        }
        Ok(())
    };

    if !trees_dir.exists() {
        eprintln!(
            "Warning: Source directory `{}` does not exist, skipping.",
            trees_dir.display()
        );
    }

    collect_files(trees_dir)?;

    Ok(Workspace { slug_exts })
}

#[derive(Debug)]
pub struct Workspace {
    pub slug_exts: HashMap<Slug, Ext>,
}
