// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Spore (@s-cerevisiae)

use std::path::{Path, PathBuf};

pub fn relative_to_current<P1, P2>(current: P1, target: P2) -> PathBuf
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    if let Some(parent) = current.as_ref().parent() {
        parent.join(target)
    } else {
        target.as_ref().to_owned()
    }
}
