// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{io::Write, sync::OnceLock};

use camino::{Utf8Path, Utf8PathBuf};
use eyre::eyre;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{
    cli::build::build_with,
    config,
    environment::{self, BuildMode},
};

#[derive(clap::Args)]
pub struct ServeCommand {
    /// Path to the configuration file (e.g., "Kodama.toml").
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG_PATH.into())]
    config: String,

    /// Enable verbose output.
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Enable verbose skip output.
    #[arg(long, default_value_t = false)]
    verbose_skip: bool,

    /// Disable live reload. 
    #[arg(short, long, default_value_t = false)]
    disable_reload: bool,
}

static LIVE_RELOAD: OnceLock<bool> = OnceLock::new();

pub fn live_reload() -> &'static bool {
    LIVE_RELOAD.get().unwrap_or(&true)
}

/// This function invoked the [`config::init_environment`] function to initialize the environment]
pub fn serve(command: &ServeCommand) -> eyre::Result<()> {
    _ = LIVE_RELOAD.set(!command.disable_reload);

    let serve_build = || -> eyre::Result<()> {
        build_with(&command.config, BuildMode::Serve, command.verbose, command.verbose_skip, false)?;
        Ok(())
    };

    serve_build()?;
    let config_file = environment::config_file();
    let config_file_canonical = config_file
        .canonicalize_utf8()
        .unwrap_or_else(|_| config_file.clone());

    print!("\x1B[2J\x1B[H");
    std::io::stdout().flush()?;

    let mut serve = spawn_serve_process()?;

    let root_dir = crate::environment::root_dir();
    let watched_paths = compose_watched_paths(
        root_dir.as_path(),
        crate::environment::trees_dir(),
        crate::environment::assets_dir(),
        config_file.clone(),
        crate::environment::theme_paths(),
    );
    watch_paths(&watched_paths, |changed_path| {
        serve_build()?;
        if should_restart_for_config_change(
            changed_path,
            config_file.as_path(),
            config_file_canonical.as_path(),
        ) {
            color_print::ceprintln!("<y>[watch] Config changed. Restarting serve process.</>");
            let _ = serve.kill();
            let _ = serve.wait();
            serve = spawn_serve_process()?;
        }
        Ok(())
    })?;

    // After watching process is done, kill the miniserve process.
    let _ = serve.kill();

    Ok(())
}

fn parse_command(command: &[String], output: Utf8PathBuf) -> eyre::Result<std::process::Command> {
    if command.is_empty() {
        return Err(eyre!("invalid `serve.command`: command list cannot be empty"));
    }

    let mut serve = std::process::Command::new(&command[0]);
    for arg in &command[1..] {
        if arg == "<output>" {
            serve.arg(&output);
            continue;
        }
        serve.arg(arg);
    }
    Ok(serve)
}

fn spawn_serve_process() -> eyre::Result<std::process::Child> {
    let command = environment::serve_command();
    let mut serve = parse_command(&command, crate::environment::output_dir())?
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    if let Some(serve_stdout) = serve.stdout.take() {
        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(serve_stdout);
            for line in reader.lines() {
                match line {
                    Ok(line) => println!("[serve] {line}"),
                    Err(err) => {
                        color_print::ceprintln!("<r>[serve] stdout read error: {err}</>");
                        break;
                    }
                }
            }
        });
    }

    if let Some(serve_stderr) = serve.stderr.take() {
        std::thread::spawn(move || {
            use std::io::{BufRead, BufReader};
            let reader = BufReader::new(serve_stderr);
            for line in reader.lines() {
                match line {
                    Ok(line) => color_print::ceprintln!("<r>[serve] Error: {line}</>"),
                    Err(err) => {
                        color_print::ceprintln!("<r>[serve] stderr read error: {err}</>");
                        break;
                    }
                }
            }
        });
    }

    Ok(serve)
}

fn compose_watched_paths(
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

fn should_restart_for_config_change(
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

/// from: https://github.com/notify-rs/notify/blob/main/examples/monitor_raw.rs#L18
fn watch_paths<P: AsRef<Utf8Path>, F>(watched_paths: &[P], mut action: F) -> eyre::Result<()>
where
    F: FnMut(&Utf8Path) -> eyre::Result<()>,
{
    let (tx, rx) = std::sync::mpsc::channel();
    let debounce = std::time::Duration::from_millis(250);
    let mut last_run = std::time::Instant::now() - debounce;

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    // All files and directories at that path and
    // below will be monitored for changes.

    print!("[watch] ");
    for watched_path in watched_paths {
        let watched_path = watched_path.as_ref();
        if !watched_path.exists() {
            color_print::ceprintln!(
                "<y>[watch] Warning: Path \"{}\" does not exist, skipping.</>",
                watched_path
            );
            continue;
        }

        let mode = watch_mode_for_path(watched_path);
        watcher.watch(watched_path.as_std_path(), mode)?;
        print!("\"{}\"  ", watched_path);
    }
    println!("\n\nPress Ctrl+C to stop watching.\n");

    for res in rx {
        match res {
            Ok(event) => {
                // Generally, we only need to listen for changes in file content `ModifyKind::Data(_)`,
                // but since notify-rs always only gets `Modify(Any)` on Windows,
                // we expand the listening scope here.
                if should_handle_watch_event(&event.kind) {
                    let now = std::time::Instant::now();
                    if now.duration_since(last_run) < debounce {
                        continue;
                    }

                    if let Some(path) = event.paths.iter().find_map(|path| path.as_path().try_into().ok()) {
                        println!("[watch] Change: {path:?}");
                        std::io::stdout().flush()?;
                        if let Err(err) = action(path) {
                            color_print::ceprintln!("<r>[watch] Rebuild failed: {}</>", err);
                        }
                        last_run = now;
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
    use super::*;
    use notify::event::{AccessKind, CreateKind, ModifyKind, RemoveKind};
    use std::fs;

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
        assert!(should_handle_watch_event(&EventKind::Modify(ModifyKind::Any)));
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

        assert_eq!(watch_mode_for_path(root.as_path()), RecursiveMode::Recursive);
        assert_eq!(watch_mode_for_path(file.as_path()), RecursiveMode::NonRecursive);

        let _ = fs::remove_dir_all(root);
    }
}
