// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

pub(super) fn scheme_name(url: &str) -> Option<String> {
    let scheme_end = url.find(':')?;
    if scheme_end == 0 {
        return None;
    }
    let first_delimiter = url.find(['/', '?', '#']).unwrap_or(url.len());
    if scheme_end > first_delimiter {
        return None;
    }
    let scheme = &url[..scheme_end];
    if scheme
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
    {
        return Some(scheme.to_ascii_lowercase());
    }
    None
}

pub(super) fn is_allowed_scheme(scheme: &str) -> bool {
    matches!(scheme, "http" | "https" | "ftp" | "mailto")
}

pub(super) fn is_unsafe_scheme(scheme: &str) -> bool {
    matches!(scheme, "javascript" | "vbscript" | "data" | "file")
}
