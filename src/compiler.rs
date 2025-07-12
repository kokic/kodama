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

use std::{
    collections::HashMap,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use eyre::{bail, eyre, Ok, WrapErr};
use parser::parse_markdown;
use section::{HTMLContent, ShallowSection};
use typst::parse_typst;
use walkdir::WalkDir;
use writer::Writer;

use crate::{
    cli::new,
    config::{self, verify_and_file_hash},
    slug::{self, Ext, Slug},
};

pub fn compile(workspace: Workspace) -> eyre::Result<()> {
    let mut shallows = HashMap::new();

    for (&slug, &ext) in &workspace.slug_exts {
        let relative_path = format!("{}.{}", slug, ext);

        let is_modified = verify_and_file_hash(&relative_path)
            .wrap_err_with(|| eyre!("Failed to verify hash of `{relative_path}`"))?;

        let entry_path = config::entry_file_path(&relative_path);
        let shallow = if !is_modified && entry_path.exists() {
            let entry_file = BufReader::new(File::open(&entry_path).wrap_err_with(|| {
                eyre!("Failed to open entry file at `{}`", entry_path.display())
            })?);
            let shallow: ShallowSection =
                serde_json::from_reader(entry_file).wrap_err_with(|| {
                    eyre!(
                        "Failed to deserialize entry file at `{}`",
                        entry_path.display()
                    )
                })?;
            shallow
        } else {
            let shallow = match ext {
                Ext::Markdown => parse_markdown(slug)
                    .wrap_err_with(|| eyre!("Failed to parse markdown file `{slug}.{ext}`"))?,
                Ext::Typst => parse_typst(slug, config::typst_root_dir())
                    .wrap_err_with(|| eyre!("Failed to parse typst file `{slug}.{ext}`"))?,
            };
            let serialized = serde_json::to_string(&shallow).unwrap();
            std::fs::write(&entry_path, serialized)
                .wrap_err_with(|| eyre!("Failed to write entry to `{}`", entry_path.display()))?;

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
        .map_or(false, |s| s.starts_with('.') || s.starts_with('_'))
}

fn find_index_file(root_dir: &PathBuf) -> eyre::Result<PathBuf> {
    let markdown_file = root_dir.join(new::DEFAULT_SECTION_PATH);
    let typst_file = root_dir.join(new::DEFAULT_SECTION_PATH_TYPST);

    let index_file = if markdown_file.exists() {
        markdown_file
    } else if typst_file.exists() {
        typst_file
    } else {
        bail!(format!(
            "Entry file not found in `{}`. Please create an entry file at `{}` or `{}`.",
            root_dir.canonicalize().unwrap().display(),
            markdown_file.display(),
            typst_file.display()
        ));
    };

    Ok(index_file)
}

fn to_slug_ext(source_dir: &Path, p: &Path) -> Option<(Slug, Ext)> {
    let p = p.strip_prefix(source_dir).unwrap_or(p);
    let ext = p.extension()?.to_str()?.parse().ok()?;
    let slug = Slug::new(slug::pretty_path(&p.with_extension("")));
    Some((slug, ext))
}

/// Collect all source file paths in workspace dir. It includes:
///
/// - `index.md` or `index.typ` as the main entry point.
/// - all `.md` and `.typ` files in the trees directory.
pub fn all_trees_source(root_dir: &PathBuf, trees_dir: &Vec<PathBuf>) -> eyre::Result<Workspace> {
    let mut slug_exts = HashMap::new();

    // TODO: Improve code in future.
    let stub_dir = Path::new("");

    // Add entry file to `slug_exts`
    let index_file = find_index_file(root_dir)?;
    let Some((slug, ext)) = to_slug_ext(&stub_dir, &index_file) else {
        bail!(eyre!(
            "Failed to parse entry file `{}`",
            index_file.display()
        ));
    };
    slug_exts.insert(slug, ext);

    let failed_to_read_dir = |dir: &Path| eyre!("Failed to read directory `{}`", dir.display());
    let file_collide = |p: &Path, e: Ext| {
        eyre!(
            "`{}` collides with `{}`",
            p.display(),
            p.with_extension(e.to_string()).display(),
        )
    };

    let mut collect_files = |source_dir: &Path| {
        Ok(
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
                            let Some((slug, ext)) = to_slug_ext(&source_dir, &path) else {
                                continue;
                            };
                            if let Some(ext) = slug_exts.insert(slug, ext) {
                                bail!(file_collide(&path, ext));
                            }
                        }
                    }
                }
            },
        )
    };

    for source_dir in trees_dir {
        if !source_dir.exists() {
            eprintln!(
                "Warning: Source directory `{}` does not exist, skipping.",
                source_dir.display()
            );
            continue;
        }
        collect_files(source_dir)?;
    }

    Ok(Workspace { slug_exts })
}

#[derive(Debug)]
pub struct Workspace {
    pub slug_exts: HashMap<Slug, Ext>,
}
