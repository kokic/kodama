// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use crate::{environment, path_utils, slug::Slug};

pub(crate) fn resolve_section_url(raw_url: &str, current_slug: Slug) -> String {
    let path = if raw_url.starts_with('/') {
        camino::Utf8PathBuf::from(raw_url)
    } else {
        path_utils::relative_to_current(current_slug.as_str(), raw_url)
    };
    let pretty = path_utils::pretty_path(path.as_path());
    if pretty.is_empty() {
        "/".to_string()
    } else {
        format!("/{pretty}")
    }
}

/// Relocate the path `/<trees>/path` to `/path`.
pub(crate) fn relocate_trees_path(path: &str) -> String {
    let trees_dir_name = environment::trees_dir_without_root();
    relocate_trees_path_with_trees_root(path, &trees_dir_name)
}

pub(crate) fn relocate_trees_path_with_trees_root(
    path: &str,
    trees_dir_without_root: &str,
) -> String {
    let trees_dir_without_root = trees_dir_without_root.trim_matches('/');
    if trees_dir_without_root.is_empty() {
        return path.to_string();
    }

    let trees = format!("/{trees_dir_without_root}");
    if path.starts_with(&trees) {
        path[trees.len()..].to_string()
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{relocate_trees_path_with_trees_root, resolve_section_url};
    use crate::slug::Slug;

    #[test]
    fn test_relocate_trees_path() {
        assert_eq!(
            relocate_trees_path_with_trees_root("/path", "trees"),
            "/path".to_string()
        );
        assert_eq!(
            relocate_trees_path_with_trees_root("/trees/path", "trees"),
            "/path".to_string()
        );
        assert_eq!(
            relocate_trees_path_with_trees_root("/docs/path", "trees"),
            "/docs/path".to_string()
        );
    }

    #[test]
    fn test_resolve_section_url_supports_root_and_relative_paths() {
        assert_eq!(
            resolve_section_url("./a.b.md", Slug::new("guide/index")),
            "/guide/a.b.md"
        );
        assert_eq!(
            resolve_section_url("../ref.md", Slug::new("guide/chapter/page")),
            "/guide/ref.md"
        );
        assert_eq!(
            resolve_section_url("/trees/root.md", Slug::new("guide/index")),
            "/trees/root.md"
        );
    }
}
