use std::time::{Duration, Instant};

use camino::{Utf8Path, Utf8PathBuf};
use notify::{EventKind, RecursiveMode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MissingPathLevel {
    Hint,
    Warning,
}

#[derive(Clone, Copy)]
pub(super) struct WatchStrategy {
    pub(super) debounce: Duration,
    pub(super) should_handle_event: fn(&EventKind) -> bool,
    pub(super) format_change_lines: fn(&mut WatchChangeFoldState, &[Utf8PathBuf]) -> Vec<String>,
    pub(super) missing_path_level: fn(&Utf8Path, &Utf8Path) -> MissingPathLevel,
}

pub(super) fn default_watch_strategy() -> WatchStrategy {
    WatchStrategy {
        debounce: Duration::from_millis(250),
        should_handle_event: should_handle_watch_event,
        format_change_lines: fold_watch_change_lines,
        missing_path_level: default_missing_path_level,
    }
}

pub(super) struct WatchBatcher {
    debounce: Duration,
    last_event_at: Option<Instant>,
    pending_changes: Vec<Utf8PathBuf>,
}

#[derive(Default)]
pub(super) struct WatchChangeFoldState {
    last_path: Option<String>,
    last_count: usize,
}

impl WatchBatcher {
    pub(super) fn new(debounce: Duration) -> Self {
        Self {
            debounce,
            last_event_at: None,
            pending_changes: Vec::new(),
        }
    }

    #[cfg(test)]
    fn with_last_event(debounce: Duration, last_event_at: Instant) -> Self {
        Self {
            debounce,
            last_event_at: Some(last_event_at),
            pending_changes: Vec::new(),
        }
    }

    pub(super) fn push_paths<I>(&mut self, paths: I, now: Instant)
    where
        I: IntoIterator<Item = Utf8PathBuf>,
    {
        let before = self.pending_changes.len();
        self.pending_changes.extend(paths);
        if self.pending_changes.len() > before {
            self.last_event_at = Some(now);
        }
    }

    pub(super) fn time_until_ready(&self, now: Instant) -> Option<Duration> {
        if self.pending_changes.is_empty() {
            return None;
        }
        let last_event_at = self.last_event_at?;
        let elapsed = now.saturating_duration_since(last_event_at);
        if elapsed >= self.debounce {
            Some(Duration::ZERO)
        } else {
            Some(self.debounce - elapsed)
        }
    }

    pub(super) fn take_ready(&mut self, now: Instant) -> Option<Vec<Utf8PathBuf>> {
        if self.pending_changes.is_empty() {
            return None;
        }
        if self
            .time_until_ready(now)
            .is_some_and(|wait| !wait.is_zero())
        {
            return None;
        }
        self.last_event_at = None;
        Some(std::mem::take(&mut self.pending_changes))
    }
}

pub(in crate::cli::serve) fn compose_watched_paths(
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

pub(super) fn should_handle_watch_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) | EventKind::Any
    )
}

pub(super) fn watch_mode_for_path(path: &Utf8Path) -> RecursiveMode {
    if path.is_file() {
        RecursiveMode::NonRecursive
    } else {
        RecursiveMode::Recursive
    }
}

pub(super) fn display_watch_path(path: &Utf8Path) -> String {
    path.as_str().replace('\\', "/")
}

pub(super) fn fold_watch_change_lines(
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

pub(super) fn default_missing_path_level(
    path: &Utf8Path,
    assets_dir: &Utf8Path,
) -> MissingPathLevel {
    if is_optional_missing_watch_path(path, assets_dir) {
        MissingPathLevel::Hint
    } else {
        MissingPathLevel::Warning
    }
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
        crate::test_io::case_dir(&format!("serve-{name}"))
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
        let mut batcher = WatchBatcher::with_last_event(debounce, anchor);

        batcher.push_paths(vec![Utf8PathBuf::from("a.md")], anchor);
        assert!(batcher
            .take_ready(anchor + Duration::from_millis(100))
            .is_none());
        assert_eq!(
            batcher.take_ready(anchor + Duration::from_millis(500)),
            Some(vec![Utf8PathBuf::from("a.md")])
        );

        batcher.push_paths(
            vec![Utf8PathBuf::from("b.md")],
            anchor + Duration::from_millis(600),
        );
        assert!(batcher
            .take_ready(anchor + Duration::from_millis(900))
            .is_none());
        assert_eq!(
            batcher.take_ready(anchor + Duration::from_millis(1100)),
            Some(vec![Utf8PathBuf::from("b.md")])
        );
    }

    #[test]
    fn test_watch_batcher_uses_trailing_debounce_window() {
        let debounce = Duration::from_millis(500);
        let anchor = Instant::now();
        let mut batcher = WatchBatcher::new(debounce);

        batcher.push_paths(vec![Utf8PathBuf::from("a.md")], anchor);
        batcher.push_paths(
            vec![Utf8PathBuf::from("b.md")],
            anchor + Duration::from_millis(300),
        );

        assert!(batcher
            .take_ready(anchor + Duration::from_millis(700))
            .is_none());
        assert_eq!(
            batcher.take_ready(anchor + Duration::from_millis(800)),
            Some(vec![Utf8PathBuf::from("a.md"), Utf8PathBuf::from("b.md")])
        );
    }
}
