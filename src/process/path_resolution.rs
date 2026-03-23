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
    use super::{relocate_trees_path, resolve_section_url};
    use crate::slug::Slug;

    #[test]
    fn test_relocate_trees_path() {
        crate::environment::mock_environment().unwrap();

        assert_eq!(relocate_trees_path("/path"), "/path".to_string());
        assert_eq!(relocate_trees_path("/trees/path"), "/path".to_string());
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
