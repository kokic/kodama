// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{
    io::Write,
    time::{Duration, Instant},
};

use camino::{Utf8Path, Utf8PathBuf};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{compiler::DirtySet, path_utils};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MissingPathLevel {
    Hint,
    Warning,
}

#[derive(Clone, Copy)]
struct WatchStrategy {
    debounce: Duration,
    should_handle_event: fn(&EventKind) -> bool,
    format_change_lines: fn(&mut WatchChangeFoldState, &[Utf8PathBuf]) -> Vec<String>,
    missing_path_level: fn(&Utf8Path, &Utf8Path) -> MissingPathLevel,
}

fn default_watch_strategy() -> WatchStrategy {
    WatchStrategy {
        debounce: Duration::from_millis(250),
        should_handle_event: should_handle_watch_event,
        format_change_lines: fold_watch_change_lines,
        missing_path_level: default_missing_path_level,
    }
}

struct WatchBatcher {
    debounce: Duration,
    last_run: Instant,
    pending_changes: Vec<Utf8PathBuf>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct WatchChangeStats {
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
pub(super) struct WatchChangeAnalysis {
    pub dirty_paths: DirtySet,
    pub stats: WatchChangeStats,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NoisePathKind {
    Temp,
    Directory,
}

#[derive(Default)]
struct WatchChangeFoldState {
    last_path: Option<String>,
    last_count: usize,
}

impl WatchBatcher {
    fn new(debounce: Duration) -> Self {
        let now = Instant::now();
        Self {
            debounce,
            last_run: now.checked_sub(debounce).unwrap_or(now),
            pending_changes: Vec::new(),
        }
    }

    #[cfg(test)]
    fn with_last_run(debounce: Duration, last_run: Instant) -> Self {
        Self {
            debounce,
            last_run,
            pending_changes: Vec::new(),
        }
    }

    fn push_paths<I>(&mut self, paths: I)
    where
        I: IntoIterator<Item = Utf8PathBuf>,
    {
        self.pending_changes.extend(paths);
    }

    fn take_ready(&mut self, now: Instant) -> Option<Vec<Utf8PathBuf>> {
        if self.pending_changes.is_empty() {
            return None;
        }
        if now.saturating_duration_since(self.last_run) < self.debounce {
            return None;
        }

        self.last_run = now;
        Some(std::mem::take(&mut self.pending_changes))
    }
}

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

fn is_path_under_dir(path: &Utf8Path, dir: &Utf8Path, dir_canonical: &Utf8Path) -> bool {
    path.starts_with(dir) || path.starts_with(dir_canonical) || {
        let canonical = canonicalize_or_self(path);
        canonical.starts_with(dir) || canonical.starts_with(dir_canonical)
    }
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

fn fold_watch_change_lines(
    state: &mut WatchChangeFoldState,
    changed_paths: &[Utf8PathBuf],
) -> Vec<String> {
    let mut grouped: Vec<(String, usize)> = Vec::new();
    for path in changed_paths {
        let path = display_watch_path(path.as_path());
        if let Some((current, count)) = grouped.last_mut() {
            if *current == path {
                *count += 1;
                continue;
            }
        }
        grouped.push((path, 1));
    }

    let mut lines = Vec::new();
    for (path, count) in grouped {
        match state.last_path.as_deref() {
            Some(last) if last == path => {
                state.last_count += count;
            }
            _ => {
                state.last_path = Some(path.clone());
                state.last_count = count;
            }
        }

        if state.last_count > 1 {
            lines.push(format!(
                "[watch] Change: \"{}\" (x{})",
                path, state.last_count
            ));
        } else {
            lines.push(format!("[watch] Change: \"{}\"", path));
        }
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

fn default_missing_path_level(path: &Utf8Path, assets_dir: &Utf8Path) -> MissingPathLevel {
    if is_optional_missing_watch_path(path, assets_dir) {
        MissingPathLevel::Hint
    } else {
        MissingPathLevel::Warning
    }
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
    matches!(ext, Some("md") | Some("typst") | Some("typ"))
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

pub(super) fn analyze_watch_changes(
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

pub(super) fn format_watch_change_stats(stats: WatchChangeStats) -> String {
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

/// from: https://github.com/notify-rs/notify/blob/main/examples/monitor_raw.rs#L18
pub(super) fn watch_paths<P: AsRef<Utf8Path>, F>(
    watched_paths: &[P],
    assets_dir: &Utf8Path,
    mut action: F,
) -> eyre::Result<()>
where
    F: FnMut(&[Utf8PathBuf]) -> eyre::Result<()>,
{
    watch_paths_with_strategy(
        watched_paths,
        assets_dir,
        default_watch_strategy(),
        &mut action,
    )
}

fn watch_paths_with_strategy<P: AsRef<Utf8Path>, F>(
    watched_paths: &[P],
    assets_dir: &Utf8Path,
    strategy: WatchStrategy,
    action: &mut F,
) -> eyre::Result<()>
where
    F: FnMut(&[Utf8PathBuf]) -> eyre::Result<()>,
{
    let (tx, rx) = std::sync::mpsc::channel();
    let mut batcher = WatchBatcher::new(strategy.debounce);
    let mut fold_state = WatchChangeFoldState::default();

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
            match (strategy.missing_path_level)(watched_path, assets_dir) {
                MissingPathLevel::Hint => {
                    color_print::ceprintln!(
                        "<dim>[watch] Hint: Optional path \"{}\" does not exist, skipping.</>",
                        watched_path_display
                    );
                }
                MissingPathLevel::Warning => {
                    color_print::ceprintln!(
                        "<y>[watch] Warning: Path \"{}\" does not exist, skipping.</>",
                        watched_path_display
                    );
                }
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
                if (strategy.should_handle_event)(&event.kind) {
                    let event_paths = event
                        .paths
                        .iter()
                        .filter_map(|path| Utf8PathBuf::from_path_buf(path.clone()).ok())
                        .collect::<Vec<_>>();
                    batcher.push_paths(event_paths);

                    let Some(changed_paths) = batcher.take_ready(Instant::now()) else {
                        continue;
                    };

                    for line in (strategy.format_change_lines)(&mut fold_state, &changed_paths) {
                        println!("{line}");
                    }
                    std::io::stdout().flush()?;
                    if let Err(err) = action(&changed_paths) {
                        // A warning color should be used here, as rebuild failures during user editing are acceptable.
                        color_print::ceprintln!("<y>[watch] Rebuild failed: {}</>", err);
                    }
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
    use std::{
        fs,
        time::{Duration, Instant},
    };

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

    #[test]
    fn test_should_handle_watch_event_kinds() {
        assert!(should_handle_watch_event(&EventKind::Any));
        assert!(should_handle_watch_event(&EventKind::Modify(
            ModifyKind::Any
        )));
        assert!(should_handle_watch_event(&EventKind::Create(
            CreateKind::Any
        )));
        assert!(should_handle_watch_event(&EventKind::Remove(
            RemoveKind::Any
        )));
        assert!(!should_handle_watch_event(&EventKind::Access(
            AccessKind::Any
        )));
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
        assert_eq!(
            display_watch_path(Utf8Path::new("site/trees/a.md")),
            "site/trees/a.md"
        );
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
        let mut state = WatchChangeFoldState::default();
        let lines = fold_watch_change_lines(&mut state, &changed);
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
        let mut state = WatchChangeFoldState::default();
        let lines = fold_watch_change_lines(&mut state, &changed);
        assert_eq!(lines, vec!["[watch] Change: \"./site/trees/a.md\" (x2)"]);
    }

    #[test]
    fn test_fold_watch_change_lines_accumulates_across_batches() {
        let mut state = WatchChangeFoldState::default();

        let first = fold_watch_change_lines(&mut state, &[Utf8PathBuf::from("site/trees/a.md")]);
        assert_eq!(first, vec!["[watch] Change: \"site/trees/a.md\""]);

        let second = fold_watch_change_lines(
            &mut state,
            &[
                Utf8PathBuf::from("site/trees/a.md"),
                Utf8PathBuf::from("site/trees/a.md"),
            ],
        );
        assert_eq!(second, vec!["[watch] Change: \"site/trees/a.md\" (x3)"]);

        let third = fold_watch_change_lines(&mut state, &[Utf8PathBuf::from("site/trees/b.md")]);
        assert_eq!(third, vec!["[watch] Change: \"site/trees/b.md\""]);

        let fourth = fold_watch_change_lines(&mut state, &[Utf8PathBuf::from("site/trees/a.md")]);
        assert_eq!(fourth, vec!["[watch] Change: \"site/trees/a.md\""]);
    }

    fn custom_change_lines(_: &mut WatchChangeFoldState, _: &[Utf8PathBuf]) -> Vec<String> {
        vec!["[watch] custom".to_string()]
    }

    fn always_warning(_: &Utf8Path, _: &Utf8Path) -> MissingPathLevel {
        MissingPathLevel::Warning
    }

    #[test]
    fn test_watch_strategy_supports_custom_change_formatter() {
        let strategy = WatchStrategy {
            format_change_lines: custom_change_lines,
            ..default_watch_strategy()
        };
        let mut fold_state = WatchChangeFoldState::default();
        let lines = (strategy.format_change_lines)(&mut fold_state, &[Utf8PathBuf::from("a.md")]);
        assert_eq!(lines, vec!["[watch] custom"]);
    }

    #[test]
    fn test_watch_strategy_supports_custom_missing_path_level() {
        let strategy = WatchStrategy {
            missing_path_level: always_warning,
            ..default_watch_strategy()
        };
        let level = (strategy.missing_path_level)(
            Utf8Path::new("site/import-meta.html"),
            Utf8Path::new("site/assets"),
        );
        assert_eq!(level, MissingPathLevel::Warning);
    }

    #[test]
    fn test_watch_batcher_respects_debounce_duration() {
        let debounce = Duration::from_millis(500);
        let anchor = Instant::now();
        let mut batcher = WatchBatcher::with_last_run(debounce, anchor);

        batcher.push_paths(vec![Utf8PathBuf::from("a.md")]);
        assert!(batcher
            .take_ready(anchor + Duration::from_millis(100))
            .is_none());
        assert_eq!(
            batcher.take_ready(anchor + Duration::from_millis(500)),
            Some(vec![Utf8PathBuf::from("a.md")])
        );

        batcher.push_paths(vec![Utf8PathBuf::from("b.md")]);
        assert!(batcher
            .take_ready(anchor + Duration::from_millis(800))
            .is_none());
        assert_eq!(
            batcher.take_ready(anchor + Duration::from_millis(1000)),
            Some(vec![Utf8PathBuf::from("b.md")])
        );
    }
}
