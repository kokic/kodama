// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{
    fs,
    io::Write,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use camino::Utf8Path;
use eyre::{eyre, WrapErr};

use crate::environment;

static ATOMIC_WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub(crate) fn sync_text_output(path: &Utf8Path, content: &str, label: &str) -> eyre::Result<()> {
    match fs::read_to_string(path.as_std_path()) {
        Ok(existing) if existing == content => return Ok(()),
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => {
            return Err(err).wrap_err_with(|| eyre!("failed to read {} from '{}'", label, path));
        }
    }

    write_text_atomically(path, content, label)
}

pub(crate) fn write_text_atomically(
    path: &Utf8Path,
    content: &str,
    label: &str,
) -> eyre::Result<()> {
    environment::create_parent_dirs(path);
    let parent = path.parent().ok_or_else(|| {
        eyre!(
            "failed to resolve parent directory for {} '{}'",
            label,
            path
        )
    })?;
    let filename = path
        .file_name()
        .ok_or_else(|| eyre!("failed to resolve filename for {} '{}'", label, path))?;
    let temp_filename = format!(
        "{filename}.tmp.{}.{}",
        std::process::id(),
        next_atomic_write_stamp()
    );
    let temp_path = parent.join(temp_filename);

    let write_result = (|| -> eyre::Result<()> {
        let mut file = fs::File::create(temp_path.as_std_path())
            .wrap_err_with(|| eyre!("failed to create temp {} '{}'", label, temp_path))?;
        file.write_all(content.as_bytes())
            .wrap_err_with(|| eyre!("failed to write temp {} '{}'", label, temp_path))?;
        file.sync_all()
            .wrap_err_with(|| eyre!("failed to sync temp {} '{}'", label, temp_path))?;
        Ok(())
    })();

    if let Err(err) = write_result {
        let _ = fs::remove_file(temp_path.as_std_path());
        return Err(err);
    }

    fs::rename(temp_path.as_std_path(), path.as_std_path()).wrap_err_with(|| {
        eyre!(
            "failed to atomically replace {} '{}' from '{}'",
            label,
            path,
            temp_path
        )
    })?;

    Ok(())
}

fn next_atomic_write_stamp() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = ATOMIC_WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{nanos}-{sequence}")
}
