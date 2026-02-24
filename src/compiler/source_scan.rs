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
///
/// **Side effect: update the `.hash` & `.svg` file of all modified `.typ` files.**
pub fn all_trees_source(trees_dir: &Utf8Path, dirty_paths: Option<&DirtySet>) -> eyre::Result<Workspace> {
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
        let compile_typst_svg = |path: &Utf8PathBuf| -> eyre::Result<()> {
            // Hashable files only include `.md` and `.typ` currently.
            if let Some("typ") = path.extension() {
                let relative = path.strip_prefix(source_dir)?;
                if let Some(dirty_paths) = dirty_paths {
                    if !dirty_paths.contains(relative) {
                        return Ok(());
                    }
                }

                let svg_url = relative.with_extension("svg");
                let svg_path = environment::output_path(&svg_url);
                if let Err(err) = crate::typst_cli::write_svg(relative, &svg_path) {
                    color_print::ceprintln!("<r>{:?} at {}</>", err, path);
                }
            }
            Ok(())
        };

        for entry in source_dir
            .read_dir_utf8()
            .wrap_err_with(|| failed_to_read_dir(source_dir))?
        {
            let path = entry
                .wrap_err_with(|| failed_to_read_dir(source_dir))?
                .into_path();

            if path.is_file() && !should_ignore_file(&path) {
                let Some((slug, ext)) = to_slug_ext(source_dir, &path) else {
                    compile_typst_svg(&path)?;
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
                            compile_typst_svg(&path)?;
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
    }

    collect_files(trees_dir)?;

    Ok(Workspace { slug_exts })
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
}
