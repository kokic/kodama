// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::collections::HashSet;
use std::fs::{self};

use camino::{Utf8Path, Utf8PathBuf};
use eyre::eyre;
use walkdir::WalkDir;

/// Synchronizes files from source directory to target directory recursively based on modification time.
/// It copies changed files from source to target and removes stale files in target.
///
/// Return `true` if any file was copied/removed.
pub fn sync_assets<P: AsRef<Utf8Path>>(source: P, target: P) -> eyre::Result<bool> {
    let source_path = source.as_ref();
    let target_path = target.as_ref();

    if !source_path.exists() {
        if target_path.exists() {
            fs::remove_dir_all(target_path)?;
            return Ok(true);
        }
        return Ok(false);
    }

    // Ensure target directory exists
    if !target_path.exists() {
        fs::create_dir_all(target_path)?;
    } else if !target_path.is_dir() {
        return Err(eyre!("target path is not a directory: {}", target_path));
    }

    let mut changed = false;
    let mut source_files: HashSet<Utf8PathBuf> = HashSet::new();

    let walkdir = WalkDir::new(source_path);
    for source_file_path in walkdir.into_iter().filter_map(|e| {
        e.ok()
            .and_then(|e| Utf8PathBuf::from_path_buf(e.into_path()).ok())
    }) {
        if !source_file_path.is_file() {
            continue;
        }

        let relative_path = source_file_path
            .strip_prefix(source_path)
            .map_err(|_| eyre::eyre!("failed to compute relative path for {}", source_file_path))?;
        source_files.insert(relative_path.to_owned());

        let target_file_path = target_path.join(relative_path);

        if let Some(parent) = target_file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let source_metadata = source_file_path.metadata()?;
        let source_mtime = source_metadata.modified()?;

        let should_copy = if target_file_path.exists() {
            let target_metadata = target_file_path.metadata()?;
            let target_mtime = target_metadata.modified()?;
            source_mtime > target_mtime || source_metadata.len() != target_metadata.len()
        } else {
            true
        };

        if should_copy {
            changed = true;
            fs::copy(source_file_path, &target_file_path)?;
        }
    }

    for target_file_path in WalkDir::new(target_path).into_iter().filter_map(|e| {
        e.ok()
            .and_then(|e| Utf8PathBuf::from_path_buf(e.into_path()).ok())
    }) {
        if !target_file_path.is_file() {
            continue;
        }

        let relative_path = target_file_path
            .strip_prefix(target_path)
            .map_err(|_| eyre::eyre!("failed to compute relative path for {}", target_file_path))?;
        if !source_files.contains(relative_path) {
            changed = true;
            fs::remove_file(target_file_path)?;
        }
    }

    let mut target_dirs: Vec<Utf8PathBuf> = WalkDir::new(target_path)
        .into_iter()
        .filter_map(|e| {
            e.ok()
                .and_then(|e| Utf8PathBuf::from_path_buf(e.into_path()).ok())
        })
        .filter(|p| p.is_dir())
        .collect();
    target_dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    for dir in target_dirs {
        if dir != target_path && dir.read_dir_utf8()?.next().is_none() {
            let _ = fs::remove_dir(&dir);
        }
    }

    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn case_dir(name: &str) -> Utf8PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("kodama-{name}-{}", fastrand::u64(..)));
        Utf8PathBuf::from_path_buf(path).expect("temp path should be valid utf8")
    }

    #[test]
    fn test_sync_assets_removes_stale_files() {
        let root = case_dir("assets-sync");
        let source = root.join("source");
        let target = root.join("target");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&target).unwrap();

        fs::write(source.join("a.txt"), "A").unwrap();
        fs::write(target.join("stale.txt"), "stale").unwrap();

        let changed = sync_assets(&source, &target).unwrap();
        assert!(changed);
        assert!(target.join("a.txt").exists());
        assert!(!target.join("stale.txt").exists());

        let changed_again = sync_assets(&source, &target).unwrap();
        assert!(!changed_again);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_sync_assets_removes_target_when_source_missing() {
        let root = case_dir("assets-missing-source");
        let source = root.join("source");
        let target = root.join("target");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("orphan.txt"), "orphan").unwrap();

        let changed = sync_assets(&source, &target).unwrap();
        assert!(changed);
        assert!(!target.exists());

        let changed_again = sync_assets(&source, &target).unwrap();
        assert!(!changed_again);

        let _ = fs::remove_dir_all(root);
    }
}
