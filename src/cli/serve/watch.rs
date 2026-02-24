// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::io::Write;

use camino::{Utf8Path, Utf8PathBuf};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{compiler::DirtySet, path_utils};

pub(super) fn compose_watched_paths(
    root_dir: &Utf8Path,
    trees_dir: Utf8PathBuf,
    assets_dir: Utf8PathBuf,
    config_file: Utf8PathBuf,
    theme_paths: Vec<Utf8PathBuf>,
) -> Vec<Utf8PathBuf> {
    let mut watched_paths = vec![
        trees_dir,
        assets_dir,
        config_file,
        root_dir.join("import-meta.html"),
        root_dir.join("import-style.html"),
        root_dir.join("import-font.html"),
        root_dir.join("import-math.html"),
    ];
    watched_paths.extend(theme_paths);
    watched_paths
}

fn canonicalize_or_self(path: &Utf8Path) -> Utf8PathBuf {
    path.canonicalize_utf8().unwrap_or_else(|_| path.to_owned())
}

pub(super) fn should_restart_for_config_change(
    changed_path: &Utf8Path,
    config_file: &Utf8Path,
    config_file_canonical: &Utf8Path,
) -> bool {
    changed_path == config_file || canonicalize_or_self(changed_path) == config_file_canonical
}

fn should_handle_watch_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) | EventKind::Any
    )
}

fn watch_mode_for_path(path: &Utf8Path) -> RecursiveMode {
    if path.is_file() {
        RecursiveMode::NonRecursive
    } else {
        RecursiveMode::Recursive
    }
}

fn display_watch_path(path: &Utf8Path) -> String {
    path.as_str().replace('\\', "/")
}

fn fold_watch_change_lines(changed_paths: &[Utf8PathBuf]) -> Vec<String> {
    let mut lines = Vec::new();
    let mut iter = changed_paths.iter();
    let Some(first) = iter.next() else {
        return lines;
    };

    let mut current = display_watch_path(first.as_path());
    let mut count = 1usize;

    for path in iter {
        let next = display_watch_path(path.as_path());
        if next == current {
            count += 1;
            continue;
        }
        if count > 1 {
            lines.push(format!("[watch] Change: \"{}\" (x{})", current, count));
        } else {
            lines.push(format!("[watch] Change: \"{}\"", current));
        }
        current = next;
        count = 1;
    }

    if count > 1 {
        lines.push(format!("[watch] Change: \"{}\" (x{})", current, count));
    } else {
        lines.push(format!("[watch] Change: \"{}\"", current));
    }
    lines
}

fn is_optional_import_watch_path(path: &Utf8Path) -> bool {
    matches!(
        path.file_name(),
        Some("import-meta.html")
            | Some("import-style.html")
            | Some("import-font.html")
            | Some("import-math.html")
    )
}

fn is_optional_missing_watch_path(path: &Utf8Path, assets_dir: &Utf8Path) -> bool {
    is_optional_import_watch_path(path) || path == assets_dir
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

pub(super) fn compose_dirty_paths(
    changed_paths: &[Utf8PathBuf],
    trees_dir: &Utf8Path,
    trees_dir_canonical: &Utf8Path,
) -> DirtySet {
    changed_paths
        .iter()
        .filter_map(|path| relative_tree_path(path.as_path(), trees_dir, trees_dir_canonical))
        .collect()
}

/// from: https://github.com/notify-rs/notify/blob/main/examples/monitor_raw.rs#L18
pub(super) fn watch_paths<P: AsRef<Utf8Path>, F>(
    watched_paths: &[P],
    assets_dir: &Utf8Path,
    mut action: F,
) -> eyre::Result<()>
where
    F: FnMut(&[Utf8PathBuf]) -> eyre::Result<()>,
{
    let (tx, rx) = std::sync::mpsc::channel();
    let debounce = std::time::Duration::from_millis(250);
    let mut last_run = std::time::Instant::now() - debounce;
    let mut pending_changes: Vec<Utf8PathBuf> = Vec::new();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    // All files and directories at that path and
    // below will be monitored for changes.

    print!("[watch] ");
    for watched_path in watched_paths {
        let watched_path = watched_path.as_ref();
        if !watched_path.exists() {
            let watched_path_display = display_watch_path(watched_path);
            if is_optional_missing_watch_path(watched_path, assets_dir) {
                color_print::ceprintln!(
                    "<dim>[watch] Hint: Optional path \"{}\" does not exist, skipping.</>",
                    watched_path_display
                );
            } else {
                color_print::ceprintln!(
                    "<y>[watch] Warning: Path \"{}\" does not exist, skipping.</>",
                    watched_path_display
                );
            }
            continue;
        }

        let mode = watch_mode_for_path(watched_path);
        watcher.watch(watched_path.as_std_path(), mode)?;
        print!("\"{}\"  ", display_watch_path(watched_path));
    }
    println!("\n\nPress Ctrl+C to stop watching.\n");

    for res in rx {
        match res {
            Ok(event) => {
                // Generally, we only need to listen for changes in file content `ModifyKind::Data(_)`,
                // but since notify-rs always only gets `Modify(Any)` on Windows,
                // we expand the listening scope here.
                if should_handle_watch_event(&event.kind) {
                    event.paths.iter().for_each(|path| {
                        if let Ok(path) = Utf8PathBuf::from_path_buf(path.clone()) {
                            pending_changes.push(path);
                        }
                    });
                    if pending_changes.is_empty() {
                        continue;
                    }

                    let now = std::time::Instant::now();
                    if now.duration_since(last_run) < debounce {
                        continue;
                    }

                    let changed_paths: Vec<Utf8PathBuf> = std::mem::take(&mut pending_changes);
                    for line in fold_watch_change_lines(&changed_paths) {
                        println!("{line}");
                    }
                    std::io::stdout().flush()?;
                    if let Err(err) = action(&changed_paths) {
                        // A warning color should be used here, as rebuild failures during user editing are acceptable.
                        color_print::ceprintln!("<y>[watch] Rebuild failed: {}</>", err);
                    }
                    last_run = now;
                }
            }
            Err(error) => {
                color_print::ceprintln!("<r>[watch] Error: {error:?}</>");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use notify::{
        event::{AccessKind, CreateKind, ModifyKind, RemoveKind},
        RecursiveMode,
    };
    use std::fs;

    use super::*;

    fn case_dir(name: &str) -> Utf8PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!("kodama-serve-{name}-{}", fastrand::u64(..)));
        Utf8PathBuf::from_path_buf(path).expect("temp path should be valid utf8")
    }

    #[test]
    fn test_compose_watched_paths_includes_imports_and_themes() {
        let root = Utf8PathBuf::from("site");
        let trees = root.join("trees");
        let assets = root.join("assets");
        let config = root.join("Kodama.toml");
        let theme = root.join("themes/theme.html");

        let watched = compose_watched_paths(
            root.as_path(),
            trees.clone(),
            assets.clone(),
            config.clone(),
            vec![theme.clone()],
        );

        assert!(watched.contains(&trees));
        assert!(watched.contains(&assets));
        assert!(watched.contains(&config));
        assert!(watched.contains(&root.join("import-meta.html")));
        assert!(watched.contains(&root.join("import-style.html")));
        assert!(watched.contains(&root.join("import-font.html")));
        assert!(watched.contains(&root.join("import-math.html")));
        assert!(watched.contains(&theme));
    }

    #[test]
    fn test_compose_dirty_paths_collects_tree_relative_files() {
        let root = Utf8PathBuf::from("site");
        let trees = root.join("trees");
        let trees_canonical = trees.clone();
        let changed = vec![trees.join("a.md"), root.join("import-style.html")];

        let dirty = compose_dirty_paths(&changed, trees.as_path(), trees_canonical.as_path());
        assert!(dirty.contains(&Utf8PathBuf::from("a.md")));
        assert!(!dirty.contains(&Utf8PathBuf::from("import-style.html")));
    }

    #[test]
    fn test_compose_dirty_paths_handles_canonicalized_tree_paths() {
        let root = case_dir("dirty-canonical");
        let trees = root.join("trees");
        let sub = trees.join("sub");
        fs::create_dir_all(&sub).unwrap();
        let file = trees.join("a.typst");
        fs::write(&file, "x").unwrap();
        let changed = vec![trees.join("sub/../a.typst")];

        let trees_canonical = trees.canonicalize_utf8().unwrap();
        let dirty = compose_dirty_paths(&changed, trees.as_path(), trees_canonical.as_path());
        assert!(dirty.contains(&Utf8PathBuf::from("a.typst")));

        let _ = fs::remove_dir_all(root);
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

    #[test]
    fn test_should_handle_watch_event_kinds() {
        assert!(should_handle_watch_event(&EventKind::Any));
        assert!(should_handle_watch_event(&EventKind::Modify(
            ModifyKind::Any
        )));
        assert!(should_handle_watch_event(&EventKind::Create(CreateKind::Any)));
        assert!(should_handle_watch_event(&EventKind::Remove(RemoveKind::Any)));
        assert!(!should_handle_watch_event(&EventKind::Access(AccessKind::Any)));
    }

    #[test]
    fn test_watch_mode_for_path_file_and_dir() {
        let root = case_dir("watch-mode");
        let file = root.join("a.txt");
        fs::create_dir_all(&root).unwrap();
        fs::write(&file, "x").unwrap();

        assert_eq!(
            watch_mode_for_path(root.as_path()),
            RecursiveMode::Recursive
        );
        assert_eq!(
            watch_mode_for_path(file.as_path()),
            RecursiveMode::NonRecursive
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn test_is_optional_import_watch_path() {
        assert!(is_optional_import_watch_path(Utf8Path::new(
            "site/import-meta.html"
        )));
        assert!(is_optional_import_watch_path(Utf8Path::new(
            "site/import-style.html"
        )));
        assert!(!is_optional_import_watch_path(Utf8Path::new(
            "site/themes/theme.html"
        )));
    }

    #[test]
    fn test_is_optional_missing_watch_path_includes_assets() {
        let assets = Utf8Path::new("site/assets");
        assert!(is_optional_missing_watch_path(
            Utf8Path::new("site/import-font.html"),
            assets
        ));
        assert!(is_optional_missing_watch_path(assets, assets));
        assert!(!is_optional_missing_watch_path(
            Utf8Path::new("site/Kodama.toml"),
            assets
        ));
    }

    #[test]
    fn test_display_watch_path_normalizes_separators() {
        assert_eq!(display_watch_path(Utf8Path::new("site/trees/a.md")), "site/trees/a.md");
        assert_eq!(
            display_watch_path(Utf8Path::new(r".\site\trees\a.md")),
            "./site/trees/a.md"
        );
    }

    #[test]
    fn test_fold_watch_change_lines_collapses_only_consecutive_duplicates() {
        let changed = vec![
            Utf8PathBuf::from("site/trees/a.md"),
            Utf8PathBuf::from("site/trees/a.md"),
            Utf8PathBuf::from("site/trees/b.md"),
            Utf8PathBuf::from("site/trees/b.md"),
            Utf8PathBuf::from("site/trees/c.md"),
            Utf8PathBuf::from("site/trees/b.md"),
        ];
        let lines = fold_watch_change_lines(&changed);
        assert_eq!(
            lines,
            vec![
                "[watch] Change: \"site/trees/a.md\" (x2)".to_string(),
                "[watch] Change: \"site/trees/b.md\" (x2)".to_string(),
                "[watch] Change: \"site/trees/c.md\"".to_string(),
                "[watch] Change: \"site/trees/b.md\"".to_string(),
            ]
        );
    }

    #[test]
    fn test_fold_watch_change_lines_normalizes_windows_separators() {
        let changed = vec![
            Utf8PathBuf::from(r".\site\trees\a.md"),
            Utf8PathBuf::from(r".\site\trees\a.md"),
        ];
        let lines = fold_watch_change_lines(&changed);
        assert_eq!(lines, vec!["[watch] Change: \"./site/trees/a.md\" (x2)"]);
    }
}
