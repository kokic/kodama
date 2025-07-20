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

use std::{collections::HashMap, fs::File, io::BufReader};

use camino::{Utf8Path, Utf8PathBuf};
use eyre::{bail, eyre, WrapErr};
use parser::parse_markdown;
use section::{HTMLContent, ShallowSection};
use typst::parse_typst;
use walkdir::WalkDir;
use writer::Writer;

use crate::{
    environment::{self, verify_and_file_hash},
    path_utils,
    slug::{Ext, Slug},
};

pub fn compile(workspace: Workspace) -> eyre::Result<()> {
    let mut shallows = HashMap::new();

    for (&slug, &ext) in &workspace.slug_exts {
        let relative_path = format!("{}.{}", slug, ext);

        let is_modified = match environment::is_serve() {
            true => verify_and_file_hash(&relative_path)
                .wrap_err_with(|| eyre!("failed to verify hash of `{relative_path}`"))?,
            false => true,
        };

        let entry_path = environment::entry_file_path(&relative_path);
        let shallow = if !is_modified && entry_path.exists() {
            let entry_file = BufReader::new(
                File::open(&entry_path)
                    .wrap_err_with(|| eyre!("failed to open entry file at `{}`", entry_path))?,
            );
            let shallow: ShallowSection = serde_json::from_reader(entry_file)
                .wrap_err_with(|| eyre!("failed to deserialize entry file at `{}`", entry_path))?;
            shallow
        } else {
            let shallow = match ext {
                Ext::Markdown => parse_markdown(slug)
                    .wrap_err_with(|| eyre!("failed to parse markdown file `{slug}.{ext}`"))?,
                Ext::Typst => parse_typst(slug, environment::typst_root_dir())
                    .wrap_err_with(|| eyre!("failed to parse typst file `{slug}.{ext}`"))?,
            };
            let serialized = serde_json::to_string(&shallow).unwrap();
            std::fs::write(&entry_path, serialized)
                .wrap_err_with(|| eyre!("failed to write entry to `{}`", entry_path))?;

            shallow
        };

        shallows.insert(slug, shallow);
    }

    let state = state::compile_all(shallows)?;
    Writer::write_needed_slugs(workspace.slug_exts.into_iter().map(|x| x.0), &state);

    Ok(())
}

pub fn should_ignored_file(path: &Utf8Path) -> bool {
    let name = path.file_name().unwrap();
    name == "README.md"
}

pub fn should_ignored_dir(path: &Utf8Path) -> bool {
    path.file_name().unwrap().starts_with(['.', '_'])
}

fn to_slug_ext(source_dir: &Utf8Path, p: &Utf8Path) -> Option<(Slug, Ext)> {
    let p = p.strip_prefix(source_dir).unwrap_or(p);
    let ext = p.extension()?.parse().ok()?;
    let slug = Slug::new(path_utils::pretty_path(&p.with_extension("")));
    Some((slug, ext))
}

/// Collect all source file paths in workspace dir.
///
/// It includes all `.md` and `.typ` files in the `trees_dir`.
pub fn all_trees_source(trees_dir: &Utf8Path) -> eyre::Result<Workspace> {
    let mut slug_exts = HashMap::new();

    let failed_to_read_dir = |dir: &Utf8Path| eyre!("failed to read directory `{}`", dir);
    let file_collide = |p: &Utf8Path, e: Ext| {
        eyre!(
            "`{}` collides with `{}`",
            p,
            p.with_extension(e.to_string()),
        )
    };

    let mut collect_files = |source_dir: &Utf8Path| {
        for entry in source_dir
            .read_dir_utf8()
            .wrap_err_with(|| failed_to_read_dir(source_dir))?
        {
            let path = entry
                .wrap_err_with(|| failed_to_read_dir(source_dir))?
                .into_path();
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
                        Utf8Path::from_path(e.path())
                            .is_some_and(|p| p.is_file() || !should_ignored_dir(p))
                    })
                {
                    let path: Utf8PathBuf = entry
                        .wrap_err_with(|| failed_to_read_dir(&path))?
                        .into_path()
                        .try_into()
                        .expect("non-UTF-8 paths are filtered out");
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
            trees_dir
        );
    }

    collect_files(trees_dir)?;

    Ok(Workspace { slug_exts })
}

#[derive(Debug)]
pub struct Workspace {
    pub slug_exts: HashMap<Slug, Ext>,
}
