// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Alias Qli (@AliasQli), Spore (@s-cerevisiae)

use std::collections::HashMap;

use camino::{Utf8Path, Utf8PathBuf};
use eyre::{bail, eyre, WrapErr};
use walkdir::WalkDir;

use crate::{
    environment, path_utils,
    slug::{Ext, Slug},
};

use super::DirtySet;

#[derive(Debug)]
pub struct Workspace {
    pub slug_exts: HashMap<Slug, Ext>,
}

pub fn should_ignore_file(path: &Utf8Path) -> bool {
    path.file_name().is_some_and(|name| name == "README.md")
}

pub fn should_ignore_dir(path: &Utf8Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.starts_with(['.', '_']))
}

fn to_slug_ext(source_dir: &Utf8Path, p: &Utf8Path) -> Option<(Slug, Ext)> {
    let p = p.strip_prefix(source_dir).unwrap_or(p);
    let ext = p.extension()?.parse().ok()?;
    let slug = Slug::new(path_utils::pretty_path(&p.with_extension("")));
    Some((slug, ext))
}

/// Collect all source file paths in `<trees>` dir.
pub fn all_trees_source(trees_dir: &Utf8Path) -> eyre::Result<Workspace> {
    all_trees_source_inner(trees_dir)
}

/// Collect all source file paths in `<trees>` dir without generating side effects.
pub fn all_trees_source_readonly(trees_dir: &Utf8Path) -> eyre::Result<Workspace> {
    all_trees_source_inner(trees_dir)
}

fn all_trees_source_inner(trees_dir: &Utf8Path) -> eyre::Result<Workspace> {
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

            if path.is_file() && !should_ignore_file(&path) {
                let Some((slug, ext)) = to_slug_ext(source_dir, &path) else {
                    continue;
                };

                if let Some(ext) = slug_exts.insert(slug, ext) {
                    bail!(file_collide(&path, ext));
                };
            } else if path.is_dir() && !should_ignore_dir(&path) {
                for entry in WalkDir::new(&path)
                    .follow_links(true)
                    .into_iter()
                    .filter_entry(|e| {
                        Utf8Path::from_path(e.path())
                            .is_some_and(|p| p.is_file() || !should_ignore_dir(p))
                    })
                {
                    let std_path = entry
                        .wrap_err_with(|| failed_to_read_dir(&path))?
                        .into_path();
                    let path = match Utf8PathBuf::from_path_buf(std_path) {
                        Ok(path) => path,
                        Err(non_utf8) => {
                            color_print::ceprintln!(
                                "<y>Warning: skipping non-UTF-8 path `{}`.</>",
                                non_utf8.display()
                            );
                            continue;
                        }
                    };
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
        color_print::ceprintln!(
            "<y>Warning: Source directory `{}` does not exist, skipping.</>",
            trees_dir
        );
        return Ok(Workspace { slug_exts });
    }

    collect_files(trees_dir)?;

    Ok(Workspace { slug_exts })
}

pub fn sync_typst_svg_assets(
    trees_dir: &Utf8Path,
    dirty_paths: Option<&DirtySet>,
) -> eyre::Result<()> {
    if !trees_dir.exists() {
        return Ok(());
    }

    match dirty_paths {
        Some(dirty_paths) => {
            for relative in dirty_paths {
                if relative.extension() != Some("typ") || is_under_ignored_dir(relative.as_path()) {
                    continue;
                }
                let full_path = trees_dir.join(relative);
                if !full_path.is_file() {
                    continue;
                }
                compile_typst_svg(trees_dir, relative.as_path());
            }
        }
        None => {
            for entry in WalkDir::new(trees_dir)
                .follow_links(true)
                .into_iter()
                .filter_entry(|e| {
                    Utf8Path::from_path(e.path())
                        .is_some_and(|p| p.is_file() || !should_ignore_dir(p))
                })
            {
                let std_path = match entry {
                    Ok(entry) => entry.into_path(),
                    Err(err) => {
                        color_print::ceprintln!(
                            "<y>Warning: failed to read path while scanning typ assets: {}</>",
                            err
                        );
                        continue;
                    }
                };
                let path = match Utf8PathBuf::from_path_buf(std_path) {
                    Ok(path) => path,
                    Err(non_utf8) => {
                        color_print::ceprintln!(
                            "<y>Warning: skipping non-UTF-8 path `{}`.</>",
                            non_utf8.display()
                        );
                        continue;
                    }
                };
                if !path.is_file() || path.extension() != Some("typ") {
                    continue;
                }
                let relative = match path.strip_prefix(trees_dir) {
                    Ok(relative) => relative,
                    Err(_) => continue,
                };
                compile_typst_svg(trees_dir, relative);
            }
        }
    }

    Ok(())
}

fn compile_typst_svg(trees_dir: &Utf8Path, relative: &Utf8Path) {
    let svg_url = relative.with_extension("svg");
    let svg_path = environment::output_path(&svg_url);
    if let Err(err) = crate::typst_cli::write_svg(relative, &svg_path) {
        let full_path = trees_dir.join(relative);
        color_print::ceprintln!("<r>{:?} at {}</>", err, full_path);
    }
}

fn is_under_ignored_dir(path: &Utf8Path) -> bool {
    let mut parent = path.parent();
    while let Some(dir) = parent {
        if should_ignore_dir(dir) {
            return true;
        }
        parent = dir.parent();
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_ignore_helpers_handle_missing_file_name() {
        let empty = Utf8Path::new("");
        assert!(!should_ignore_file(empty));
        assert!(!should_ignore_dir(empty));
    }

    #[test]
    fn test_should_ignore_helpers_match_expected_names() {
        assert!(should_ignore_file(Utf8Path::new("README.md")));
        assert!(!should_ignore_file(Utf8Path::new("docs.md")));

        assert!(should_ignore_dir(Utf8Path::new(".git")));
        assert!(should_ignore_dir(Utf8Path::new("_tmp")));
        assert!(!should_ignore_dir(Utf8Path::new("trees")));
    }

    #[test]
    fn test_all_trees_source_readonly_returns_empty_workspace_when_trees_missing() {
        let missing = crate::test_io::case_dir("missing-trees");

        let workspace = all_trees_source_readonly(missing.as_path()).expect("scan should succeed");
        assert!(workspace.slug_exts.is_empty());
    }

    #[test]
    fn test_sync_typst_svg_assets_ignores_missing_tree_root() {
        let missing = crate::test_io::case_dir("missing-typ-assets");
        assert!(sync_typst_svg_assets(missing.as_path(), None).is_ok());
    }

    #[test]
    fn test_is_under_ignored_dir_detects_internal_helper_dirs() {
        assert!(is_under_ignored_dir(Utf8Path::new("_lib/kodama.typ")));
        assert!(is_under_ignored_dir(Utf8Path::new("docs/.cache/a.typ")));
        assert!(!is_under_ignored_dir(Utf8Path::new("docs/math/a.typ")));
        assert!(!is_under_ignored_dir(Utf8Path::new("a.typ")));
    }
}
