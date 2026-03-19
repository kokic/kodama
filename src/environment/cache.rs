// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::io::ErrorKind;

use camino::{Utf8Path, Utf8PathBuf};
use eyre::{eyre, WrapErr};

const CACHE_VERSION_FILE: &str = "version";
const CACHE_SCHEMA_VERSION: &str = "schema-v1";

fn cache_version_value() -> String {
    format!(
        "kodama:{}:{}",
        env!("CARGO_PKG_VERSION"),
        CACHE_SCHEMA_VERSION
    )
}

fn cache_version_path() -> Utf8PathBuf {
    super::get_cache_dir().join(CACHE_VERSION_FILE)
}

fn read_cache_version(path: &Utf8Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|content| content.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn remove_dir_if_exists(path: &Utf8Path) -> eyre::Result<()> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err).wrap_err_with(|| eyre!("failed to remove directory `{}`", path)),
    }
}

pub fn ensure_cache_version() -> eyre::Result<()> {
    let cache_dir = super::get_cache_dir();
    std::fs::create_dir_all(cache_dir.as_std_path())
        .wrap_err_with(|| eyre!("failed to create cache directory `{}`", cache_dir))?;

    let version_path = cache_version_path();
    let expected = cache_version_value();
    let current = read_cache_version(version_path.as_path());

    if current.as_deref() == Some(expected.as_str()) {
        return Ok(());
    }

    remove_dir_if_exists(super::hash_dir().as_path())?;
    remove_dir_if_exists(super::entry_dir().as_path())?;

    super::create_parent_dirs(version_path.as_path());
    std::fs::write(version_path.as_std_path(), expected.as_bytes())
        .wrap_err_with(|| eyre!("failed to write cache version file `{}`", version_path))?;

    if current.is_some() {
        color_print::ceprintln!(
            "<dim>[cache] Cache layout changed. Cleared \"{}\" and \"{}\".</>",
            super::HASH_DIR_NAME,
            super::ENTRY_DIR_NAME
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_ensure_cache_version_keeps_existing_cache_when_version_matches() {
        let root = crate::test_io::case_dir("env-cache-keep");
        fs::create_dir_all(root.as_std_path()).unwrap();

        super::super::with_test_environment(root.clone(), super::super::BuildMode::Build, || {
            ensure_cache_version().unwrap();
            let hash_file = super::super::hash_file_path("a.md");
            let entry_file = super::super::entry_file_path("a.md");
            fs::write(hash_file.as_std_path(), "1").unwrap();
            fs::write(entry_file.as_std_path(), "{}").unwrap();

            ensure_cache_version().unwrap();

            assert!(hash_file.exists());
            assert!(entry_file.exists());
        });

        let _ = fs::remove_dir_all(root.as_std_path());
    }

    #[test]
    fn test_ensure_cache_version_clears_entry_and_hash_on_mismatch() {
        let root = crate::test_io::case_dir("env-cache-mismatch");
        fs::create_dir_all(root.as_std_path()).unwrap();

        super::super::with_test_environment(root.clone(), super::super::BuildMode::Build, || {
            let hash_file = super::super::hash_file_path("a.md");
            let entry_file = super::super::entry_file_path("a.md");
            fs::write(hash_file.as_std_path(), "1").unwrap();
            fs::write(entry_file.as_std_path(), "{}").unwrap();

            let version_path = cache_version_path();
            super::super::create_parent_dirs(version_path.as_path());
            fs::write(version_path.as_std_path(), "legacy-version").unwrap();

            ensure_cache_version().unwrap();

            assert!(!hash_file.exists());
            assert!(!entry_file.exists());
            let current = fs::read_to_string(version_path.as_std_path()).unwrap();
            assert_eq!(current.trim(), cache_version_value());
        });

        let _ = fs::remove_dir_all(root.as_std_path());
    }
}
