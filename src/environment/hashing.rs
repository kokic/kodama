// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use camino::Utf8Path;
use eyre::{eyre, Context};

/// Return is file modified i.e. is hash updated.
pub fn is_hash_updated<P: AsRef<Utf8Path>>(content: &str, hash_path: P) -> (bool, u64) {
    let mut hasher = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(&content, &mut hasher);
    let current_hash = std::hash::Hasher::finish(&hasher);

    let history_hash = std::fs::read_to_string(hash_path.as_ref())
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0); // no file / invalid hash: 0

    (current_hash != history_hash, current_hash)
}

/// Checks whether the file has been modified by comparing its current hash with the stored hash.
/// If the file is modified, updates the stored hash to reflect the latest state.
pub fn verify_and_file_hash<P: AsRef<Utf8Path>>(relative_path: P) -> eyre::Result<bool> {
    if *crate::cli::build::no_cache_enabled() {
        return Ok(true);
    }

    let root_dir = super::trees_dir();
    let full_path = root_dir.join(&relative_path);
    let hash_path = super::hash_file_path(&relative_path);

    let content = std::fs::read_to_string(&full_path)
        .wrap_err_with(|| eyre!("failed to read file `{}`", full_path))?;
    let (is_modified, current_hash) = is_hash_updated(&content, &hash_path);
    if is_modified {
        std::fs::write(&hash_path, current_hash.to_string())
            .wrap_err_with(|| eyre!("failed to write file `{}`", hash_path))?;
    }
    Ok(is_modified)
}

/// Checks whether the content has been modified by comparing its current hash with the stored hash.
/// If the content is modified, updates the stored hash to reflect the latest state.
pub fn verify_update_hash<P: AsRef<Utf8Path>>(
    path: P,
    content: &str,
) -> Result<bool, std::io::Error> {
    if *crate::cli::build::no_cache_enabled() {
        return Ok(true);
    }

    let hash_path = super::hash_file_path(path.as_ref());
    let (is_modified, current_hash) = is_hash_updated(content, &hash_path);
    if is_modified {
        std::fs::write(&hash_path, current_hash.to_string())?;
    }

    Ok(is_modified)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn test_is_hash_updated_returns_modified_when_hash_file_missing() {
        let root = crate::test_io::case_dir("env-hash-missing");
        let hash_path = root.join("missing.hash");
        let (is_modified, _) = is_hash_updated("content", hash_path.as_path());
        assert!(is_modified);
    }

    #[test]
    fn test_is_hash_updated_returns_modified_for_invalid_hash_history() {
        let root = crate::test_io::case_dir("env-hash-invalid");
        fs::create_dir_all(root.as_std_path()).unwrap();
        let hash_path = root.join("history.hash");
        fs::write(hash_path.as_std_path(), "not-a-number").unwrap();

        let (is_modified, _) = is_hash_updated("content", hash_path.as_path());
        assert!(is_modified);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_is_hash_updated_returns_unmodified_for_matching_hash() {
        let root = crate::test_io::case_dir("env-hash-matching");
        fs::create_dir_all(root.as_std_path()).unwrap();
        let hash_path = root.join("matching.hash");
        let (_, current_hash) = is_hash_updated("content", hash_path.as_path());
        fs::write(hash_path.as_std_path(), current_hash.to_string()).unwrap();

        let (is_modified, _) = is_hash_updated("content", hash_path.as_path());
        assert!(!is_modified);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_verify_update_hash_roundtrip_detects_changes() {
        let root = crate::test_io::case_dir("env-hash-roundtrip");
        fs::create_dir_all(root.as_std_path()).unwrap();

        super::super::with_test_environment(root.clone(), super::super::BuildMode::Publish, || {
            let relative = "hash-tests/a.md";
            assert!(verify_update_hash(relative, "v1").unwrap());
            assert!(!verify_update_hash(relative, "v1").unwrap());
            assert!(verify_update_hash(relative, "v2").unwrap());

            let hash_path = super::super::hash_file_path(relative);
            assert!(hash_path.exists());
        });

        let _ = fs::remove_dir_all(root);
    }
}
