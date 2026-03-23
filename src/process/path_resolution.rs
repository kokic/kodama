// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use crate::environment;

/// Relocate the path `/<trees>/path` to `/path`.
pub(crate) fn relocate_trees_path(path: &str) -> String {
    let trees = environment::trees_dir_without_root();
    let trees = format!("/{trees}");
    if path.starts_with(&trees) {
        path[trees.len()..].to_string()
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::relocate_trees_path;

    #[test]
    fn test_relocate_trees_path() {
        crate::environment::mock_environment().unwrap();

        assert_eq!(relocate_trees_path("/path"), "/path".to_string());
        assert_eq!(relocate_trees_path("/trees/path"), "/path".to_string());
    }
}
