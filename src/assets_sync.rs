// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use fs_extra::file::{copy, CopyOptions};
use std::fs::{self};
use std::io;
use std::path::Path;
use walkdir::WalkDir;

/// Synchronizes files from source directory to target directory recursively based on modification time.
/// If all files in source (including subdirectories) have the same modification time as in target,
/// the function exits early. If any file in source is newer, it is copied to target. 
/// 
/// Return `true` if all files have same modification time. 
pub fn sync_assets<P: AsRef<Path>>(source: P , target: P) -> io::Result<bool> {
    let source_path = source.as_ref();
    let target_path = target.as_ref();

    if !source_path.exists() {
        return Ok(true)
    }

    // Ensure target directory exists
    if !target_path.exists() {
        fs::create_dir_all(target_path)?;
    } else if !target_path.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Target path `{}` is not a directory", target_path.display()),
        ));
    }

    // Flag to track if all files have same modification time
    let mut all_same_mtime = true;

    let walkdir = WalkDir::new(source_path);
    for entry in walkdir.into_iter().filter_map(|e| e.ok()) {
        let source_file_path = entry.path();
        if !source_file_path.is_file() {
            continue;
        }

        let relative_path = source_file_path.strip_prefix(source_path).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Failed to compute relative path",
            )
        })?;
        let target_file_path = target_path.join(relative_path);

        if let Some(parent) = target_file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let source_metadata = source_file_path.metadata()?;
        let source_mtime = source_metadata.modified()?;

        // Check if target file exists and compare modification times
        if target_file_path.exists() {
            let target_metadata = target_file_path.metadata()?;
            let target_mtime = target_metadata.modified()?;

            // If source file is newer, copy it
            if source_mtime > target_mtime {
                all_same_mtime = false;
                let options = CopyOptions::new().overwrite(true);
                copy(&source_file_path, &target_file_path, &options)
                    .map(|_| ())
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            }
        } else {
            // Target file does not exist, copy source file
            all_same_mtime = false;
            let options = CopyOptions::new().overwrite(true);
            copy(&source_file_path, &target_file_path, &options)
                .map(|_| ())
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        }
    }

    Ok(all_same_mtime)
}
