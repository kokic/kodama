use std::{io::Write, sync::mpsc::RecvTimeoutError, time::Instant};

use camino::{Utf8Path, Utf8PathBuf};
use notify::{Config, RecommendedWatcher, Watcher};

use super::strategy::{
    default_watch_strategy, display_watch_path, watch_mode_for_path, MissingPathLevel,
    WatchBatcher, WatchChangeFoldState, WatchStrategy,
};

/// from: https://github.com/notify-rs/notify/blob/main/examples/monitor_raw.rs#L18
pub(in crate::cli::serve) fn watch_paths<P: AsRef<Utf8Path>, F>(
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

    loop {
        let now = Instant::now();
        let res = match batcher.time_until_ready(now) {
            Some(wait) => rx.recv_timeout(wait),
            None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };

        match res {
            Ok(Ok(event)) => {
                // Generally, we only need to listen for changes in file content `ModifyKind::Data(_)`,
                // but since notify-rs always only gets `Modify(Any)` on Windows,
                // we expand the listening scope here.
                if let Some(paths) = collect_event_paths(event, strategy) {
                    batcher.push_paths(paths, Instant::now());
                }

                while let Ok(Ok(event)) = rx.try_recv() {
                    if let Some(paths) = collect_event_paths(event, strategy) {
                        batcher.push_paths(paths, Instant::now());
                    }
                }
                while let Ok(Err(error)) = rx.try_recv() {
                    color_print::ceprintln!("<r>[watch] Error: {error:?}</>");
                }
            }
            Ok(Err(error)) => {
                color_print::ceprintln!("<r>[watch] Error: {error:?}</>");
            }
            Err(RecvTimeoutError::Timeout) => {
                let Some(changed_paths) = batcher.take_ready(Instant::now()) else {
                    continue;
                };
                process_batch(&changed_paths, &mut fold_state, strategy, action)?;
            }
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(())
}

fn process_batch<F>(
    changed_paths: &[Utf8PathBuf],
    fold_state: &mut WatchChangeFoldState,
    strategy: WatchStrategy,
    action: &mut F,
) -> eyre::Result<()>
where
    F: FnMut(&[Utf8PathBuf]) -> eyre::Result<()>,
{
    for line in (strategy.format_change_lines)(fold_state, changed_paths) {
        println!("{line}");
    }
    std::io::stdout().flush()?;
    if let Err(err) = action(changed_paths) {
        // A warning color should be used here, as rebuild failures during user editing are acceptable.
        color_print::ceprintln!("<y>[watch] Rebuild failed: {}</>", err);
    }
    Ok(())
}

fn collect_event_paths(event: notify::Event, strategy: WatchStrategy) -> Option<Vec<Utf8PathBuf>> {
    if !(strategy.should_handle_event)(&event.kind) {
        return None;
    }
    Some(
        event
            .paths
            .iter()
            .filter_map(|path| Utf8PathBuf::from_path_buf(path.clone()).ok())
            .collect::<Vec<_>>(),
    )
}
