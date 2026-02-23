// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{
    fs::{self, create_dir_all},
    sync::{OnceLock, RwLock},
};

use camino::{Utf8Path, Utf8PathBuf};
use eyre::{eyre, Context};

use crate::{
    config::{self, build::FooterMode, kodama, toc, Config},
    path_utils,
    slug::Slug,
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

    /// Serve mode for the `kodama serve` command.
    Serve,
}

const DEFAULT_IMPORT_FONT_HTML: &str = include_str!("include/import-font.html");
const DEFAULT_IMPORT_MATH_HTML: &str = include_str!("include/import-math.html");

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

pub fn import_meta_html() -> String {
    fs::read_to_string(root_dir().join("import-meta.html")).unwrap_or_default()
}

pub fn import_style_html() -> String {
    fs::read_to_string(root_dir().join("import-style.html")).unwrap_or_default()
}

pub fn import_fonts_html() -> String {
    fs::read_to_string(root_dir().join("import-font.html"))
        .unwrap_or_else(|_| DEFAULT_IMPORT_FONT_HTML.to_string())
}

pub fn import_math_html() -> String {
    fs::read_to_string(root_dir().join("import-math.html"))
        .unwrap_or_else(|_| DEFAULT_IMPORT_MATH_HTML.to_string())
}

pub fn is_serve() -> bool {
    with_environment(|env| matches!(env.build_mode, BuildMode::Serve))
}

pub fn is_build() -> bool {
    with_environment(|env| matches!(env.build_mode, BuildMode::Build))
}

pub fn is_short_slug() -> bool {
    with_config(|cfg| cfg.build.short_slug)
}

pub fn typst_root_dir() -> Utf8PathBuf {
    with_config(|cfg| cfg.build.typst_root.clone().into())
}

pub fn trees_dir_without_root() -> String {
    with_config(|cfg| cfg.kodama.trees.clone())
}

pub fn trees_dir() -> Utf8PathBuf {
    root_dir().join(trees_dir_without_root())
}

pub fn theme_paths() -> Vec<Utf8PathBuf> {
    let root = root_dir();
    with_config(|cfg| {
        cfg.kodama
            .themes
            .iter()
            .map(|theme| root.join(theme))
            .collect()
    })
}

pub fn build_dir() -> String {
    with_config(|cfg| cfg.build.output.clone())
}

pub fn serve_dir() -> String {
    with_config(|cfg| cfg.serve.output.clone())
}

pub fn output_dir() -> Utf8PathBuf {
    let root = root_dir();
    let output = if is_build() { build_dir() } else { serve_dir() };
    root.join(output)
}

pub fn indexes_path(output_dir: &Utf8Path) -> Utf8PathBuf {
    output_dir.join("kodama.json")
}

pub fn base_url_raw() -> String {
    with_config(|cfg| cfg.kodama.base_url.clone())
}

pub fn base_url() -> String {
    with_environment(|env| match env.build_mode {
        BuildMode::Build => env.config.kodama.base_url.clone(),
        BuildMode::Serve => kodama::DEFAULT_BASE_URL.to_string(),
    })
}

pub fn is_toc_left() -> bool {
    match with_config(|cfg| cfg.toc.placement) {
        toc::TocPlacement::Left => true,
        toc::TocPlacement::Right => false,
    }
}

pub fn is_toc_sticky() -> bool {
    with_config(|cfg| cfg.toc.sticky)
}

pub fn is_toc_mobile_sticky() -> bool {
    with_config(|cfg| cfg.toc.mobile_sticky)
}

pub fn toc_max_width() -> String {
    with_config(|cfg| cfg.toc.max_width.clone())
}

pub fn get_edit_text() -> String {
    with_config(|cfg| cfg.text.edit.clone())
}

pub fn get_toc_text() -> String {
    with_config(|cfg| cfg.text.toc.clone())
}

pub fn get_footer_references_text() -> String {
    with_config(|cfg| cfg.text.references.clone())
}

pub fn get_footer_backlinks_text() -> String {
    with_config(|cfg| cfg.text.backlinks.clone())
}

pub fn footer_mode() -> FooterMode {
    with_config(|cfg| cfg.build.footer_mode)
}

pub fn inline_css() -> bool {
    with_config(|cfg| cfg.build.inline_css)
}

pub fn asref() -> bool {
    with_config(|cfg| cfg.build.asref)
}

pub fn deploy_edit_url() -> Option<String> {
    with_config(|cfg| cfg.build.edit.clone())
}

pub fn editor_url() -> Option<String> {
    with_config(|cfg| cfg.serve.edit.clone())
}

pub fn serve_command() -> Vec<String> {
    with_config(|cfg| cfg.serve.command.clone())
}

pub fn get_cache_dir() -> Utf8PathBuf {
    root_dir().join(CACHE_DIR_NAME)
}

pub fn assets_dir() -> Utf8PathBuf {
    let assets = with_config(|cfg| cfg.kodama.assets.clone());
    root_dir().join(assets)
}

/// URL keep posix style, so the type of return value is [`String`].
pub fn full_url<P: AsRef<Utf8Path>>(path: P) -> String {
    let base_url = base_url();
    let path = path_utils::pretty_path(path.as_ref());
    if let Some(stripped) = path.strip_prefix("/") {
        return format!("{base_url}{stripped}");
    } else if let Some(stripped) = path.strip_prefix("./") {
        return format!("{base_url}{stripped}");
    }
    format!("{base_url}{path}")
}

pub fn full_html_url(slug: Slug) -> String {
    let pretty_urls = with_config(|cfg| cfg.build.pretty_urls);
    let page_suffix = to_page_suffix(pretty_urls);
    full_url(format!("{}{}", slug, page_suffix))
}

pub fn input_path<P: AsRef<Utf8Path>>(path: P) -> Utf8PathBuf {
    let mut filepath: Utf8PathBuf = trees_dir();
    filepath.push(path);
    filepath
}

pub fn create_parent_dirs<P: AsRef<Utf8Path>>(path: P) {
    let Some(parent_dir) = path.as_ref().parent() else {
        return;
    };
    if !parent_dir.exists() {
        if let Err(err) = create_dir_all(parent_dir) {
            color_print::ceprintln!(
                "<y>Warning: failed to create parent directory `{}`: {}</>",
                parent_dir,
                err
            );
        }
    }
}

pub fn auto_create_dir_path<P: AsRef<Utf8Path>>(paths: Vec<P>) -> Utf8PathBuf {
    let mut filepath: Utf8PathBuf = root_dir();
    for path in paths {
        filepath.push(path);
    }
    create_parent_dirs(&filepath);
    filepath
}

pub fn output_path<P: AsRef<Utf8Path>>(path: P) -> Utf8PathBuf {
    let dir = output_dir();
    let dir = dir.as_path();
    let path = path.as_ref();
    auto_create_dir_path(vec![dir, path])
}

/// Return the output HTML path `<output_dir>/<path>.html` for the given section.
/// e.g. `/path/to/index.md` will return `<output_dir>/path/to/index.html`.
///
/// If the directory does not exist, it will be created.
#[allow(dead_code)]
pub fn output_html_path<P: AsRef<Utf8Path>>(path: P) -> Utf8PathBuf {
    let mut output_path = output_dir();
    output_path.push(path);
    output_path.set_extension("html");
    create_parent_dirs(&output_path);
    output_path
}

pub fn hash_dir() -> Utf8PathBuf {
    get_cache_dir().join(HASH_DIR_NAME)
}

/// Return the hash file path `<hash_dir>/<path>.hash` for the given file or directory.
/// e.g. `/path/to/index.md` will return `<hash_dir>/path/to/index.md.hash`.
///
/// If the directory does not exist, it will be created.
pub fn hash_file_path<P: AsRef<Utf8Path>>(path: P) -> Utf8PathBuf {
    let mut hash_path = hash_dir();
    hash_path.push(path);
    let ext = hash_path
        .extension()
        .map(|ext| format!("{ext}.hash"))
        .unwrap_or_else(|| "hash".to_string());
    hash_path.set_extension(ext);
    create_parent_dirs(&hash_path);
    hash_path
}

pub fn entry_dir() -> Utf8PathBuf {
    get_cache_dir().join(ENTRY_DIR_NAME)
}

/// Return the hash file path `<hash_dir>/<path>.hash` for the given file or directory.
/// e.g. `/path/to/index.md` will return `<entry_dir>/path/to/index.md.entry`.
///
/// If the directory does not exist, it will be created.
pub fn entry_file_path<P: AsRef<Utf8Path>>(path: P) -> Utf8PathBuf {
    let mut entry_path = entry_dir();
    entry_path.push(path);
    let ext = entry_path
        .extension()
        .map(|ext| format!("{ext}.entry"))
        .unwrap_or_else(|| "entry".to_string());
    entry_path.set_extension(ext);
    create_parent_dirs(&entry_path);
    entry_path
}

/// Return is file modified i.e. is hash updated.
pub fn is_hash_updated<P: AsRef<Utf8Path>>(content: &str, hash_path: P) -> (bool, u64) {
    let mut hasher = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(&content, &mut hasher);
    let current_hash = std::hash::Hasher::finish(&hasher);

    let history_hash = std::fs::read_to_string(hash_path.as_ref())
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0); // no file / invalid hash: 0

    (current_hash != history_hash, current_hash)
}

/// Checks whether the file has been modified by comparing its current hash with the stored hash.
/// If the file is modified, updates the stored hash to reflect the latest state.
pub fn verify_and_file_hash<P: AsRef<Utf8Path>>(relative_path: P) -> eyre::Result<bool> {
    if *crate::cli::build::enable_no_cache() {
        return Ok(true);
    }

    let root_dir = trees_dir();
    let full_path = root_dir.join(&relative_path);
    let hash_path = hash_file_path(&relative_path);

    let content = std::fs::read_to_string(&full_path)
        .wrap_err_with(|| eyre::eyre!("failed to read file `{}`", full_path))?;
    let (is_modified, current_hash) = is_hash_updated(&content, &hash_path);
    if is_modified {
        std::fs::write(&hash_path, current_hash.to_string())
            .wrap_err_with(|| eyre::eyre!("failed to write file `{}`", hash_path))?;
    }
    Ok(is_modified)
}

/// Checks whether the content has been modified by comparing its current hash with the stored hash.
/// If the content is modified, updates the stored hash to reflect the latest state.
pub fn verify_update_hash<P: AsRef<Utf8Path>>(
    path: P,
    content: &str,
) -> Result<bool, std::io::Error> {
    if *crate::cli::build::enable_no_cache() {
        return Ok(true);
    }

    let hash_path = hash_file_path(path.as_ref());
    let (is_modified, current_hash) = is_hash_updated(content, &hash_path);
    if is_modified {
        std::fs::write(&hash_path, current_hash.to_string())?;
    }

    Ok(is_modified)
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
