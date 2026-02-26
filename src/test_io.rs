// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use camino::Utf8PathBuf;

/// Build an isolated filesystem path for IO-heavy tests under `.local/test-io/`.
pub(crate) fn case_dir(name: &str) -> Utf8PathBuf {
    let base = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(".local")
        .join("test-io");
    let _ = std::fs::create_dir_all(&base);

    let path = base.join(format!("kodama-{name}-{}", fastrand::u64(..)));
    Utf8PathBuf::from_path_buf(path).expect("test io path should be valid utf8")
}
