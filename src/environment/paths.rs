// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::fs::create_dir_all;

use camino::{Utf8Path, Utf8PathBuf};

use crate::{path_utils, slug::Slug};

use super::{ENTRY_DIR_NAME, HASH_DIR_NAME};

/// URL keep posix style, so the type of return value is [`String`].
pub fn full_url<P: AsRef<Utf8Path>>(path: P) -> String {
    let base_url = super::base_url();
    let path = path_utils::pretty_path(path.as_ref());
    if let Some(stripped) = path.strip_prefix("/") {
        return format!("{base_url}{stripped}");
    } else if let Some(stripped) = path.strip_prefix("./") {
        return format!("{base_url}{stripped}");
    }
    format!("{base_url}{path}")
}

pub fn full_html_url(slug: Slug) -> String {
    let pretty_urls = super::with_config(|cfg| cfg.build.pretty_urls);
    let page_suffix = super::to_page_suffix(pretty_urls);
    full_url(format!("{}{}", slug, page_suffix))
}

pub fn input_path<P: AsRef<Utf8Path>>(path: P) -> Utf8PathBuf {
    let mut filepath: Utf8PathBuf = super::trees_dir();
    filepath.push(path);
    filepath
}

pub fn create_parent_dirs<P: AsRef<Utf8Path>>(path: P) {
    let Some(parent_dir) = path.as_ref().parent() else {
        return;
    };
    if !parent_dir.exists() {
        if let Err(err) = create_dir_all(parent_dir) {
            color_print::ceprintln!(
                "<y>Warning: failed to create parent directory `{}`: {}</>",
                parent_dir,
                err
            );
        }
    }
}

pub fn auto_create_dir_path<P: AsRef<Utf8Path>>(paths: Vec<P>) -> Utf8PathBuf {
    let mut filepath: Utf8PathBuf = super::root_dir();
    for path in paths {
        filepath.push(path);
    }
    create_parent_dirs(&filepath);
    filepath
}

pub fn output_path<P: AsRef<Utf8Path>>(path: P) -> Utf8PathBuf {
    let dir = super::output_dir();
    let dir = dir.as_path();
    let path = path.as_ref();
    auto_create_dir_path(vec![dir, path])
}

pub fn hash_dir() -> Utf8PathBuf {
    super::get_cache_dir().join(HASH_DIR_NAME)
}

/// Return the hash file path `<hash_dir>/<path>.hash` for the given file or directory.
/// e.g. `/path/to/index.md` will return `<hash_dir>/path/to/index.md.hash`.
///
/// If the directory does not exist, it will be created.
pub fn hash_file_path<P: AsRef<Utf8Path>>(path: P) -> Utf8PathBuf {
    let mut hash_path = hash_dir();
    hash_path.push(path);
    let ext = hash_path
        .extension()
        .map(|ext| format!("{ext}.hash"))
        .unwrap_or_else(|| "hash".to_string());
    hash_path.set_extension(ext);
    create_parent_dirs(&hash_path);
    hash_path
}

pub fn entry_dir() -> Utf8PathBuf {
    super::get_cache_dir().join(ENTRY_DIR_NAME)
}

/// Return the hash file path `<hash_dir>/<path>.hash` for the given file or directory.
/// e.g. `/path/to/index.md` will return `<entry_dir>/path/to/index.md.entry`.
///
/// If the directory does not exist, it will be created.
pub fn entry_file_path<P: AsRef<Utf8Path>>(path: P) -> Utf8PathBuf {
    let mut entry_path = entry_dir();
    entry_path.push(path);
    let ext = entry_path
        .extension()
        .map(|ext| format!("{ext}.entry"))
        .unwrap_or_else(|| "entry".to_string());
    entry_path.set_extension(ext);
    create_parent_dirs(&entry_path);
    entry_path
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_create_parent_dirs_creates_missing_directories() {
        let root = crate::test_io::case_dir("env-paths-parent");
        let target = root.join("a/b/c/file.txt");
        create_parent_dirs(target.as_path());
        assert!(target.parent().is_some_and(|parent| parent.exists()));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_full_url_normalizes_leading_prefixes() {
        let root = crate::test_io::case_dir("env-paths-full-url");
        fs::create_dir_all(root.as_std_path()).unwrap();

        super::super::with_test_environment(root.clone(), super::super::BuildMode::Publish, || {
            let base = super::super::base_url();
            assert_eq!(full_url("/notes/a"), format!("{base}notes/a"));
            assert_eq!(full_url("./notes/a"), format!("{base}notes/a"));
            assert_eq!(full_url("notes/a"), format!("{base}notes/a"));
        });

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_hash_and_entry_paths_preserve_original_extension_suffix() {
        let root = crate::test_io::case_dir("env-paths-hash-entry");
        fs::create_dir_all(root.as_std_path()).unwrap();

        super::super::with_test_environment(root.clone(), super::super::BuildMode::Publish, || {
            let hash = hash_file_path("nested/a.b.md");
            let entry = entry_file_path("nested/a.b.md");

            assert!(hash.as_str().contains("a.b.md.hash"));
            assert!(entry.as_str().contains("a.b.md.entry"));
            assert!(hash.parent().is_some_and(|parent| parent.exists()));
            assert!(entry.parent().is_some_and(|parent| parent.exists()));
        });

        let _ = fs::remove_dir_all(root);
    }
}
