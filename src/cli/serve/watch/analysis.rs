// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use camino::{Utf8Path, Utf8PathBuf};

use crate::{compiler::DirtySet, path_utils};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(in crate::cli::serve) struct WatchChangeStats {
    pub total_paths: usize,
    pub tree_source_paths: usize,
    pub tree_dependency_paths: usize,
    pub asset_paths: usize,
    pub global_paths: usize,
    pub ignored_temp_paths: usize,
    pub ignored_directory_paths: usize,
}

impl WatchChangeStats {
    pub fn has_effective_changes(self) -> bool {
        self.tree_source_paths > 0
            || self.tree_dependency_paths > 0
            || self.asset_paths > 0
            || self.global_paths > 0
    }
}

#[derive(Debug, Default)]
pub(in crate::cli::serve) struct WatchChangeAnalysis {
    pub dirty_paths: DirtySet,
    pub stats: WatchChangeStats,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NoisePathKind {
    Temp,
    Directory,
}

fn canonicalize_or_self(path: &Utf8Path) -> Utf8PathBuf {
    path.canonicalize_utf8().unwrap_or_else(|_| path.to_owned())
}

fn is_path_under_dir(path: &Utf8Path, dir: &Utf8Path, dir_canonical: &Utf8Path) -> bool {
    path.starts_with(dir) || path.starts_with(dir_canonical) || {
        let canonical = canonicalize_or_self(path);
        canonical.starts_with(dir) || canonical.starts_with(dir_canonical)
    }
}

pub(in crate::cli::serve) fn should_restart_for_config_change(
    changed_path: &Utf8Path,
    config_file: &Utf8Path,
    config_file_canonical: &Utf8Path,
) -> bool {
    changed_path == config_file || canonicalize_or_self(changed_path) == config_file_canonical
}

fn strip_tree_prefix(path: &Utf8Path, trees_dir: &Utf8Path) -> Option<Utf8PathBuf> {
    let relative = path.strip_prefix(trees_dir).ok()?;
    let pretty = Utf8PathBuf::from(path_utils::pretty_path(relative));
    if pretty.as_str().is_empty() || pretty.as_str() == "." {
        return None;
    }
    Some(pretty)
}

fn relative_tree_path(
    path: &Utf8Path,
    trees_dir: &Utf8Path,
    trees_dir_canonical: &Utf8Path,
) -> Option<Utf8PathBuf> {
    strip_tree_prefix(path, trees_dir)
        .or_else(|| strip_tree_prefix(path, trees_dir_canonical))
        .or_else(|| {
            let canonical = canonicalize_or_self(path);
            strip_tree_prefix(canonical.as_path(), trees_dir)
                .or_else(|| strip_tree_prefix(canonical.as_path(), trees_dir_canonical))
        })
}

fn is_source_extension(ext: Option<&str>) -> bool {
    matches!(ext, Some("md") | Some("typst"))
}

fn is_temp_like_path(path: &Utf8Path) -> bool {
    let Some(file_name) = path.file_name() else {
        return false;
    };
    let lower = file_name.to_ascii_lowercase();
    lower == ".ds_store"
        || file_name.starts_with(".#")
        || (file_name.starts_with('#') && file_name.ends_with('#'))
        || file_name.ends_with('~')
        || lower.ends_with(".tmp")
        || lower.ends_with(".temp")
        || lower.ends_with(".swp")
        || lower.ends_with(".swx")
        || lower.ends_with(".bak")
        || lower.ends_with(".crdownload")
        || lower.contains("__jb_tmp__")
        || lower.contains("__jb_old__")
}

fn classify_noise_path(path: &Utf8Path) -> Option<NoisePathKind> {
    if is_temp_like_path(path) {
        return Some(NoisePathKind::Temp);
    }

    if path.is_dir() {
        return Some(NoisePathKind::Directory);
    }

    None
}

pub(in crate::cli::serve) fn analyze_watch_changes(
    changed_paths: &[Utf8PathBuf],
    trees_dir: &Utf8Path,
    trees_dir_canonical: &Utf8Path,
    assets_dir: &Utf8Path,
    assets_dir_canonical: &Utf8Path,
) -> WatchChangeAnalysis {
    let mut dirty_paths = DirtySet::new();
    let mut stats = WatchChangeStats::default();

    for path in changed_paths {
        stats.total_paths += 1;
        if let Some(noise_kind) = classify_noise_path(path.as_path()) {
            match noise_kind {
                NoisePathKind::Temp => stats.ignored_temp_paths += 1,
                NoisePathKind::Directory => stats.ignored_directory_paths += 1,
            }
            continue;
        }

        if let Some(relative) = relative_tree_path(path.as_path(), trees_dir, trees_dir_canonical) {
            if is_source_extension(relative.extension()) {
                stats.tree_source_paths += 1;
            } else {
                stats.tree_dependency_paths += 1;
            }
            dirty_paths.insert(relative);
            continue;
        }

        if is_path_under_dir(path.as_path(), assets_dir, assets_dir_canonical) {
            stats.asset_paths += 1;
            continue;
        }

        stats.global_paths += 1;
    }

    WatchChangeAnalysis { dirty_paths, stats }
}

pub(in crate::cli::serve) fn format_watch_change_stats(stats: WatchChangeStats) -> String {
    format!(
        "[watch] Stats: total={}, tree_source={}, tree_dependency={}, assets={}, global={}, ignored_temp={}, ignored_dir={}",
        stats.total_paths,
        stats.tree_source_paths,
        stats.tree_dependency_paths,
        stats.asset_paths,
        stats.global_paths,
        stats.ignored_temp_paths,
        stats.ignored_directory_paths
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn case_dir(name: &str) -> Utf8PathBuf {
        crate::test_io::case_dir(&format!("serve-{name}"))
    }

    #[test]
    fn test_analyze_watch_changes_collects_tree_relative_files() {
        let root = Utf8PathBuf::from("site");
        let trees = root.join("trees");
        let trees_canonical = trees.clone();
        let assets = root.join("assets");
        let assets_canonical = assets.clone();
        let changed = vec![trees.join("a.md"), root.join("import-style.html")];

        let analysis = analyze_watch_changes(
            &changed,
            trees.as_path(),
            trees_canonical.as_path(),
            assets.as_path(),
            assets_canonical.as_path(),
        );
        assert!(analysis.dirty_paths.contains(&Utf8PathBuf::from("a.md")));
        assert!(!analysis
            .dirty_paths
            .contains(&Utf8PathBuf::from("import-style.html")));
    }

    #[test]
    fn test_analyze_watch_changes_handles_canonicalized_tree_paths() {
        let root = case_dir("dirty-canonical");
        let trees = root.join("trees");
        let assets = root.join("assets");
        let sub = trees.join("sub");
        fs::create_dir_all(&sub).unwrap();
        let file = trees.join("a.typst");
        fs::write(&file, "x").unwrap();
        let changed = vec![trees.join("sub/../a.typst")];

        let trees_canonical = trees.canonicalize_utf8().unwrap();
        let analysis = analyze_watch_changes(
            &changed,
            trees.as_path(),
            trees_canonical.as_path(),
            assets.as_path(),
            assets.as_path(),
        );
        assert!(analysis.dirty_paths.contains(&Utf8PathBuf::from("a.typst")));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_analyze_watch_changes_ignores_temp_and_directory_events() {
        let root = case_dir("dirty-filter");
        let trees = root.join("trees");
        let assets = root.join("assets");
        let dir = trees.join("sub");
        fs::create_dir_all(dir.as_std_path()).unwrap();

        let changed = vec![
            trees.join("index.md"),
            trees.join(".index.md.swp"),
            dir.clone(),
        ];

        let trees_canonical = trees.canonicalize_utf8().unwrap();
        let analysis = analyze_watch_changes(
            &changed,
            trees.as_path(),
            trees_canonical.as_path(),
            assets.as_path(),
            assets.as_path(),
        );
        assert!(analysis
            .dirty_paths
            .contains(&Utf8PathBuf::from("index.md")));
        assert!(!analysis
            .dirty_paths
            .contains(&Utf8PathBuf::from(".index.md.swp")));
        assert!(!analysis.dirty_paths.contains(&Utf8PathBuf::from("sub")));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_analyze_watch_changes_classifies_paths_and_filters_noise() {
        let root = case_dir("analyze");
        let trees = root.join("trees");
        let assets = root.join("assets");
        let themes = root.join("themes");
        let tree_dir = trees.join("sub");
        fs::create_dir_all(tree_dir.as_std_path()).unwrap();
        fs::create_dir_all(assets.as_std_path()).unwrap();
        fs::create_dir_all(themes.as_std_path()).unwrap();

        let changed = vec![
            trees.join("index.md"),
            trees.join("includes/snippet.txt"),
            trees.join(".index.md.swp"),
            tree_dir.clone(),
            assets.join("logo.svg"),
            themes.join("theme.html"),
        ];

        let trees_canonical = trees.canonicalize_utf8().unwrap();
        let assets_canonical = assets.canonicalize_utf8().unwrap();
        let analysis = analyze_watch_changes(
            &changed,
            trees.as_path(),
            trees_canonical.as_path(),
            assets.as_path(),
            assets_canonical.as_path(),
        );

        assert!(analysis
            .dirty_paths
            .contains(&Utf8PathBuf::from("index.md")));
        assert!(analysis
            .dirty_paths
            .contains(&Utf8PathBuf::from("includes/snippet.txt")));
        assert_eq!(
            analysis.stats,
            WatchChangeStats {
                total_paths: 6,
                tree_source_paths: 1,
                tree_dependency_paths: 1,
                asset_paths: 1,
                global_paths: 1,
                ignored_temp_paths: 1,
                ignored_directory_paths: 1,
            }
        );
        assert!(analysis.stats.has_effective_changes());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_format_watch_change_stats_contains_all_counters() {
        let line = format_watch_change_stats(WatchChangeStats {
            total_paths: 7,
            tree_source_paths: 2,
            tree_dependency_paths: 1,
            asset_paths: 1,
            global_paths: 1,
            ignored_temp_paths: 1,
            ignored_directory_paths: 1,
        });
        assert_eq!(
            line,
            "[watch] Stats: total=7, tree_source=2, tree_dependency=1, assets=1, global=1, ignored_temp=1, ignored_dir=1"
        );
    }

    #[test]
    fn test_should_restart_for_config_change_exact_path() {
        let config = Utf8PathBuf::from("site/Kodama.toml");
        assert!(should_restart_for_config_change(
            config.as_path(),
            config.as_path(),
            config.as_path()
        ));
    }

    #[test]
    fn test_should_restart_for_config_change_canonical_match() {
        let root = case_dir("canonical");
        let sub = root.join("sub");
        fs::create_dir_all(&sub).unwrap();
        let config = root.join("Kodama.toml");
        fs::write(&config, "[kodama]\n").unwrap();
        let changed = root.join("sub/../Kodama.toml");

        let config_canonical = config.canonicalize_utf8().unwrap();
        assert!(should_restart_for_config_change(
            changed.as_path(),
            config.as_path(),
            config_canonical.as_path()
        ));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_should_restart_for_config_change_other_file() {
        let config = Utf8PathBuf::from("site/Kodama.toml");
        let changed = Utf8PathBuf::from("site/trees/index.md");
        assert!(!should_restart_for_config_change(
            changed.as_path(),
            config.as_path(),
            config.as_path()
        ));
    }
}
