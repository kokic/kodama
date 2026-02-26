// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Alias Qli (@AliasQli), Spore (@s-cerevisiae)

use std::collections::HashSet;
use std::{fs::File, io::BufReader};

use camino::{Utf8Path, Utf8PathBuf};
use eyre::{eyre, WrapErr};
use walkdir::WalkDir;

use crate::{
    environment, path_utils,
    slug::{Ext, Slug},
};

use super::Workspace;
use super::{section::UnresolvedSection, CachedSourceEntry};

pub(super) fn source_from_entry_relative_path(
    entry_relative_path: &Utf8Path,
) -> Option<(Utf8PathBuf, Slug, Ext)> {
    let entry_relative_path = path_utils::pretty_path(entry_relative_path);
    let source_relative_path = entry_relative_path.strip_suffix(".entry")?;
    let source_relative_path = Utf8PathBuf::from(source_relative_path);
    let ext = source_relative_path.extension()?.parse().ok()?;
    let slug = Slug::new(path_utils::pretty_path(
        &source_relative_path.with_extension(""),
    ));
    Some((source_relative_path, slug, ext))
}

fn same_ext(a: Ext, b: Ext) -> bool {
    matches!(
        (a, b),
        (Ext::Markdown, Ext::Markdown) | (Ext::Typst, Ext::Typst)
    )
}

pub(super) fn hash_cache_path_no_create(
    hash_dir: &Utf8Path,
    source_relative_path: &Utf8Path,
) -> Utf8PathBuf {
    let mut hash_path = hash_dir.join(source_relative_path);
    let ext = hash_path
        .extension()
        .map(|ext| format!("{ext}.hash"))
        .unwrap_or_else(|| "hash".to_string());
    hash_path.set_extension(ext);
    hash_path
}

pub(super) fn remove_file_if_exists(path: &Utf8Path) -> eyre::Result<bool> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err).wrap_err_with(|| eyre!("failed to remove file `{}`", path)),
    }
}

pub(super) fn cleanup_stale_slug_artifacts_with_paths(
    workspace: &Workspace,
    entry_dir: &Utf8Path,
    hash_dir: &Utf8Path,
    output_dir: &Utf8Path,
) -> eyre::Result<HashSet<Slug>> {
    let mut stale_slugs = HashSet::new();
    if !entry_dir.exists() {
        return Ok(stale_slugs);
    }

    for entry in WalkDir::new(entry_dir).follow_links(true).into_iter() {
        let std_path = entry
            .wrap_err_with(|| eyre!("failed to read cached entry directory `{}`", entry_dir))?
            .into_path();
        let entry_path = match Utf8PathBuf::from_path_buf(std_path) {
            Ok(path) => path,
            Err(non_utf8) => {
                color_print::ceprintln!(
                    "<y>Warning: skipping non-UTF-8 cache path `{}`.</>",
                    non_utf8.display()
                );
                continue;
            }
        };
        if !entry_path.is_file() || entry_path.extension() != Some("entry") {
            continue;
        }

        let relative_entry = entry_path
            .strip_prefix(entry_dir)
            .unwrap_or(entry_path.as_path());
        let Some((source_relative, slug, ext)) = source_from_entry_relative_path(relative_entry)
        else {
            continue;
        };

        if workspace
            .slug_exts
            .get(&slug)
            .copied()
            .is_some_and(|current_ext| same_ext(current_ext, ext))
        {
            continue;
        }

        let stale_entry_slugs = read_cached_slugs(entry_path.as_path(), slug);
        stale_slugs.extend(stale_entry_slugs.iter().copied());

        let _ = remove_file_if_exists(entry_path.as_path())?;

        let hash_path = hash_cache_path_no_create(hash_dir, source_relative.as_path());
        let _ = remove_file_if_exists(hash_path.as_path())?;

        for stale_slug in stale_entry_slugs {
            let output_html = output_dir.join(format!("{}.html", stale_slug));
            let _ = remove_file_if_exists(output_html.as_path())?;
        }
    }

    Ok(stale_slugs)
}

fn read_cached_slugs(entry_path: &Utf8Path, fallback_slug: Slug) -> Vec<Slug> {
    let read_bundle = || -> eyre::Result<Vec<Slug>> {
        let entry_file = BufReader::new(
            File::open(entry_path)
                .wrap_err_with(|| eyre!("failed to open cached entry `{}`", entry_path))?,
        );
        let cached: CachedSourceEntry = serde_json::from_reader(entry_file)
            .wrap_err_with(|| eyre!("failed to deserialize cached entry `{}`", entry_path))?;
        Ok(cached
            .sections
            .into_iter()
            .map(|section| section.slug)
            .collect())
    };

    if let Ok(slugs) = read_bundle() {
        if !slugs.is_empty() {
            return slugs;
        }
    }

    let read_legacy = || -> eyre::Result<Vec<Slug>> {
        let entry_file = BufReader::new(
            File::open(entry_path)
                .wrap_err_with(|| eyre!("failed to reopen cached entry `{}`", entry_path))?,
        );
        let section: UnresolvedSection = serde_json::from_reader(entry_file)
            .wrap_err_with(|| eyre!("failed to deserialize legacy entry `{}`", entry_path))?;
        let slug = section.slug().unwrap_or(fallback_slug);
        Ok(vec![slug])
    };

    match read_legacy() {
        Ok(slugs) if !slugs.is_empty() => slugs,
        _ => vec![fallback_slug],
    }
}

pub(super) fn cleanup_stale_slug_artifacts(workspace: &Workspace) -> eyre::Result<HashSet<Slug>> {
    cleanup_stale_slug_artifacts_with_paths(
        workspace,
        environment::entry_dir().as_path(),
        environment::hash_dir().as_path(),
        environment::output_dir().as_path(),
    )
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs};

    use super::*;

    #[test]
    fn test_source_from_entry_relative_path_parses_slug_and_extension() {
        let relative = Utf8Path::new("foo/bar.md.entry");
        let (source_relative, slug, ext) = source_from_entry_relative_path(relative).unwrap();
        assert_eq!(source_relative, Utf8PathBuf::from("foo/bar.md"));
        assert_eq!(slug, Slug::new("foo/bar"));
        assert!(matches!(ext, Ext::Markdown));
    }

    #[test]
    fn test_cleanup_stale_slug_artifacts_removes_stale_output_and_cache() {
        let base = crate::test_io::case_dir("cleanup-stale");
        let entry_dir = base.join("entry");
        let hash_dir = base.join("hash");
        let output_dir = base.join("output");
        fs::create_dir_all(&entry_dir).unwrap();
        fs::create_dir_all(&hash_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let stale_source = Utf8PathBuf::from("old.md");
        let mut stale_entry = entry_dir.join(&stale_source);
        stale_entry.set_extension("md.entry");
        let stale_hash = hash_cache_path_no_create(hash_dir.as_path(), stale_source.as_path());
        let stale_output = output_dir.join("old.html");
        fs::create_dir_all(stale_entry.parent().unwrap()).unwrap();
        fs::create_dir_all(stale_hash.parent().unwrap()).unwrap();
        fs::write(&stale_entry, "{}").unwrap();
        fs::write(&stale_hash, "1").unwrap();
        fs::write(&stale_output, "<html/>").unwrap();

        let keep_source = Utf8PathBuf::from("keep.md");
        let mut keep_entry = entry_dir.join(&keep_source);
        keep_entry.set_extension("md.entry");
        let keep_hash = hash_cache_path_no_create(hash_dir.as_path(), keep_source.as_path());
        let keep_output = output_dir.join("keep.html");
        fs::write(&keep_entry, "{}").unwrap();
        fs::write(&keep_hash, "1").unwrap();
        fs::write(&keep_output, "<html/>").unwrap();

        let mut slug_exts = HashMap::new();
        slug_exts.insert(Slug::new("keep"), Ext::Markdown);
        let workspace = Workspace { slug_exts };

        let stale = cleanup_stale_slug_artifacts_with_paths(
            &workspace,
            entry_dir.as_path(),
            hash_dir.as_path(),
            output_dir.as_path(),
        )
        .unwrap();

        assert!(stale.contains(&Slug::new("old")));
        assert!(!stale.contains(&Slug::new("keep")));
        assert!(!stale_entry.exists());
        assert!(!stale_hash.exists());
        assert!(!stale_output.exists());
        assert!(keep_entry.exists());
        assert!(keep_hash.exists());
        assert!(keep_output.exists());

        let _ = fs::remove_dir_all(base);
    }
}
