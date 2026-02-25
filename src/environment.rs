// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::sync::{OnceLock, RwLock};

use camino::Utf8PathBuf;
use eyre::eyre;

use crate::{
    config::{self, Config},
    path_utils,
};

mod cache;
mod config_access;
mod hashing;
mod imports;
mod paths;

pub use cache::ensure_cache_version;
pub use config_access::{
    asref, assets_dir, base_url, base_url_raw, deploy_edit_url, editor_url, footer_mode,
    get_cache_dir, get_edit_text, get_footer_backlinks_text, get_footer_references_text,
    get_toc_text, graph_path, indexes_path, inline_css, is_short_slug, is_toc_left,
    is_toc_mobile_sticky, is_toc_sticky, output_dir, reload_marker_path, serve_command,
    theme_paths, toc_max_width, trees_dir, trees_dir_without_root, typst_root_dir,
};
pub use hashing::{verify_and_file_hash, verify_update_hash};
pub use imports::{import_fonts_html, import_math_html, import_meta_html, import_style_html};
pub use paths::{
    create_parent_dirs, entry_dir, entry_file_path, full_html_url, full_url, hash_dir,
    hash_file_path, input_path, output_path,
};

pub struct Environment {
    /// Specifies the project root path.
    ///
    /// Please note that this value should always be automatically derived from
    /// the location of the toml configuration file.
    pub root: Utf8PathBuf,
    pub config_file: Utf8PathBuf,
    pub config: Config,
    pub build_mode: BuildMode,
}

static ENVIRONMENT: OnceLock<RwLock<Environment>> = OnceLock::new();

fn default_environment() -> Environment {
    Environment {
        root: "./".into(),
        config_file: crate::config::DEFAULT_CONFIG_PATH.into(),
        config: Config::default(),
        build_mode: BuildMode::Build,
    }
}

fn read_environment<R>(lock: &RwLock<Environment>, f: impl FnOnce(&Environment) -> R) -> R {
    match lock.read() {
        Ok(env) => f(&env),
        Err(poisoned) => {
            color_print::ceprintln!(
                "<y>Warning: environment read lock is poisoned; continuing with recovered state.</>"
            );
            let env = poisoned.into_inner();
            f(&env)
        }
    }
}

fn write_environment(lock: &RwLock<Environment>, environment: Environment) {
    match lock.write() {
        Ok(mut env) => {
            *env = environment;
        }
        Err(poisoned) => {
            color_print::ceprintln!(
                "<y>Warning: environment write lock is poisoned; replacing with recovered state.</>"
            );
            let mut env = poisoned.into_inner();
            *env = environment;
        }
    }
}

fn environment_lock(warn_if_uninitialized: bool) -> &'static RwLock<Environment> {
    if warn_if_uninitialized && ENVIRONMENT.get().is_none() {
        color_print::ceprintln!(
            "<y>Warning: environment accessed before initialization; using default configuration.</>"
        );
    }
    ENVIRONMENT.get_or_init(|| RwLock::new(default_environment()))
}

fn update_environment(environment: Environment) {
    let lock = environment_lock(false);
    write_environment(lock, environment);
}

fn with_environment<R>(f: impl FnOnce(&Environment) -> R) -> R {
    let lock = environment_lock(true);
    read_environment(lock, f)
}

fn with_config<R>(f: impl FnOnce(&Config) -> R) -> R {
    with_environment(|env| f(&env.config))
}

#[cfg(test)]
pub(super) fn with_test_environment<R>(
    root: Utf8PathBuf,
    build_mode: BuildMode,
    f: impl FnOnce() -> R,
) -> R {
    static TEST_ENV_MUTEX: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    let lock = TEST_ENV_MUTEX.get_or_init(|| std::sync::Mutex::new(()));
    let _guard = lock.lock().expect("test env mutex must be lockable");

    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            update_environment(default_environment());
        }
    }

    let _reset = Reset;
    update_environment(Environment {
        root: root.clone(),
        config_file: root.join(crate::config::DEFAULT_CONFIG_PATH),
        config: Config::default(),
        build_mode,
    });
    f()
}

pub fn init_environment(toml_file: Utf8PathBuf, build_mode: BuildMode) -> eyre::Result<()> {
    let toml_file = config::find_config(toml_file)?;

    let (root, _file_name) = path_utils::split_file_name(&toml_file)
        .ok_or_else(|| eyre!("invalid config path `{}`: path cannot be empty", toml_file))?;
    let toml = std::fs::read_to_string(&toml_file)?;

    update_environment(Environment {
        root: root.to_owned(),
        config_file: toml_file,
        config: config::parse_config(&toml)?,
        build_mode,
    });
    Ok(())
}

/// Mock environment for testing purposes.
#[allow(dead_code)]
pub fn mock_environment() -> eyre::Result<()> {
    update_environment(default_environment());
    Ok(())
}

#[derive(Clone, Copy)]
pub enum BuildMode {
    /// Build mode for the `kodama build` command.
    Build,

    /// Check mode for the `kodama check` command.
    Check,

    /// Serve mode for the `kodama serve` command.
    Serve,
}

pub const CACHE_DIR_NAME: &str = ".cache";
pub const HASH_DIR_NAME: &str = "hash";
pub const ENTRY_DIR_NAME: &str = "entry";

pub fn to_page_suffix(pretty_urls: bool) -> String {
    let page_suffix = match pretty_urls {
        true => "",
        false => ".html",
    };
    page_suffix.into()
}

pub fn root_dir() -> Utf8PathBuf {
    with_environment(|env| env.root.clone())
}

pub fn config_file() -> Utf8PathBuf {
    with_environment(|env| env.config_file.clone())
}

pub fn is_serve() -> bool {
    with_environment(|env| matches!(env.build_mode, BuildMode::Serve))
}

pub fn is_build() -> bool {
    with_environment(|env| matches!(env.build_mode, BuildMode::Build))
}

pub fn is_check() -> bool {
    with_environment(|env| matches!(env.build_mode, BuildMode::Check))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, RwLock};

    use super::*;

    fn poison_read_lock(lock: Arc<RwLock<Environment>>) {
        let _ = std::thread::spawn(move || {
            let _guard = lock.write().unwrap();
            panic!("poison lock");
        })
        .join();
    }

    #[test]
    fn test_read_environment_recovers_from_poisoned_lock() {
        let lock = Arc::new(RwLock::new(default_environment()));
        poison_read_lock(lock.clone());

        let root = read_environment(&lock, |env| env.root.clone());
        assert_eq!(root, Utf8PathBuf::from("./"));
    }

    #[test]
    fn test_write_environment_recovers_from_poisoned_lock() {
        let lock = Arc::new(RwLock::new(default_environment()));
        poison_read_lock(lock.clone());

        write_environment(
            &lock,
            Environment {
                root: Utf8PathBuf::from("site"),
                config_file: Utf8PathBuf::from("Kodama.toml"),
                config: Config::default(),
                build_mode: BuildMode::Serve,
            },
        );

        let (root, mode) = read_environment(&lock, |env| (env.root.clone(), env.build_mode));
        assert_eq!(root, Utf8PathBuf::from("site"));
        assert!(matches!(mode, BuildMode::Serve));
    }
}
