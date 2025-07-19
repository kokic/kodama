// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Spore (@s-cerevisiae)

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};

pub fn relative_to_current<P1, P2>(current: P1, target: P2) -> Utf8PathBuf
where
    P1: AsRef<Utf8Path>,
    P2: AsRef<Utf8Path>,
{
    if let Some(parent) = current.as_ref().parent() {
        parent.join(target)
    } else {
        target.as_ref().to_owned()
    }
}

pub fn pretty_path(path: &Utf8Path) -> String {
    let mut segments = Vec::new();
    for c in path.components() {
        match c {
            Utf8Component::Prefix(_) | Utf8Component::RootDir | Utf8Component::CurDir => (),
            Utf8Component::ParentDir => {
                segments.pop();
            }
            Utf8Component::Normal(_) => segments.push(c.as_str()),
        }
    }
    segments.join("/")
}
