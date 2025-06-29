// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{
    fs::{self, create_dir_all},
    hash::Hash,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{LazyLock, Mutex},
};

use eyre::Context;
use walkdir::WalkDir;

use crate::slug::Slug;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum FooterMode {
    Link,
    Embed,
}

#[derive(Debug)]
pub struct ParseFooterModeError;

impl FromStr for FooterMode {
    type Err = ParseFooterModeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "link" => Ok(FooterMode::Link),
            "embed" => Ok(FooterMode::Embed),
            _ => Err(ParseFooterModeError),
        }
    }
}

impl ToString for FooterMode {
    fn to_string(&self) -> String {
        match self {
            FooterMode::Link => "link".into(),
            FooterMode::Embed => "embed".into(),
        }
    }
}

pub struct CompileConfig<S> {
    pub root_dir: S,
    pub output_dir: S,
    pub assets_dir: S,
    pub base_url: S,
    pub page_suffix: S,
    pub short_slug: bool,
    pub footer_mode: FooterMode,

    /// `false`: This is very useful for users who want to modify existing styles or configure other themes.
    pub disable_export_css: bool,

    /// URL prefix for opening files in the editor.
    pub edit: Option<S>,
}

impl CompileConfig<&'static str> {
    pub const fn default() -> CompileConfig<&'static str> {
        CompileConfig {
            root_dir: "./",
            output_dir: "./publish",
            assets_dir: "./assets",
            base_url: "/",
            page_suffix: "",
            short_slug: true,
            footer_mode: FooterMode::Link,
            disable_export_css: true,
            edit: None,
        }
    }
}

impl CompileConfig<String> {
    const fn empty() -> CompileConfig<String> {
        CompileConfig {
            root_dir: String::new(),
            output_dir: String::new(),
            assets_dir: String::new(),
            base_url: String::new(),
            page_suffix: String::new(),
            short_slug: true,
            footer_mode: FooterMode::Link,
            disable_export_css: true,
            edit: None,
        }
    }

    pub fn new<'a>(
        root_dir: String,
        output_dir: String,
        assets_dir: String,
        base_url: String,
        disable_pretty_urls: bool,
        short_slug: bool,
        footer_mode: FooterMode,
        disable_export_css: bool,
        edit: Option<String>,
    ) -> CompileConfig<String> {
        CompileConfig {
            root_dir,
            output_dir,
            assets_dir,
            base_url: normalize_base_url(&base_url),
            page_suffix: to_page_suffix(disable_pretty_urls),
            short_slug,
            footer_mode,
            disable_export_css,
            edit,
        }
    }
}

pub static DEFAULT_CONFIG: CompileConfig<&'static str> = CompileConfig::default();
pub static CONFIG: Mutex<CompileConfig<String>> = Mutex::new(CompileConfig::empty());

pub static CUSTOM_META_HTML: LazyLock<String> = LazyLock::new(|| {
    std::fs::read_to_string(root_dir().join("import-meta.html")).unwrap_or_default()
});

pub static CUSTOM_STYLE_HTML: LazyLock<String> = LazyLock::new(|| {
    std::fs::read_to_string(root_dir().join("import-style.html")).unwrap_or_default()
});

pub static CUSTOM_FONTS_HTML: LazyLock<String> = LazyLock::new(|| {
    fs::read_to_string(root_dir().join("import-fonts.html"))
        .unwrap_or(include_str!("include/import-fonts.html").to_string())
});

pub static CUSTOM_MATH_HTML: LazyLock<String> = LazyLock::new(|| {
    fs::read_to_string(root_dir().join("import-math.html"))
        .unwrap_or(include_str!("include/import-math.html").to_string())
});

pub fn lock_config() -> std::sync::MutexGuard<'static, CompileConfig<std::string::String>> {
    CONFIG.lock().unwrap()
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Blink {
    pub source: String,
    pub target: String,
}

pub const CACHE_DIR_NAME: &str = ".cache";
pub const BUFFER_FILE_NAME: &str = "buffer";
pub const HASH_DIR_NAME: &str = "hash";
pub const ENTRY_DIR_NAME: &str = "entry";

pub fn mutex_set<T>(source: &Mutex<T>, target: T) {
    let mut guard = source.lock().unwrap();
    *guard = target;
}

pub fn to_page_suffix(disable_pretty_urls: bool) -> String {
    let page_suffix = match disable_pretty_urls {
        true => ".html",
        false => "",
    };
    page_suffix.into()
}

pub fn normalize_base_url(base_url: &str) -> String {
    match base_url.ends_with("/") {
        true => base_url.to_string(),
        false => format!("{}/", base_url),
    }
}

pub fn is_short_slug() -> bool {
    lock_config().short_slug
}

pub fn root_dir() -> PathBuf {
    lock_config().root_dir.to_string().into()
}

pub fn output_dir() -> PathBuf {
    lock_config().output_dir.to_string().into()
}

pub fn base_url() -> String {
    lock_config().base_url.to_string()
}

pub fn footer_mode() -> FooterMode {
    lock_config().footer_mode.clone()
}

pub fn disable_export_css() -> bool {
    lock_config().disable_export_css
}

pub fn editor_url() -> Option<String> {
    lock_config().edit.clone()
}

pub fn get_cache_dir() -> PathBuf {
    root_dir().join(CACHE_DIR_NAME)
}

pub fn assets_dir() -> PathBuf {
    root_dir().join(lock_config().assets_dir.to_string())
}

/// URL keep posix style, so the type of return value is [`String`].
pub fn full_url<P: AsRef<Path>>(path: P) -> String {
    let path = crate::slug::pretty_path(path.as_ref());
    if path.starts_with("/") {
        return format!("{}{}", base_url(), path[1..].to_string());
    } else if path.starts_with("./") {
        return format!("{}{}", base_url(), path[2..].to_string());
    }
    format!("{}{}", base_url(), path)
}

pub fn full_html_url(slug: Slug) -> String {
    full_url(&format!("{}{}", slug, lock_config().page_suffix))
}

/// Convert `path` to `./{path}` or `path`.
///
/// This function keep posix style for the path, so it will return a [`String`].
pub fn relativize(path: &str) -> String {
    match path.starts_with("/") {
        true => format!(".{}", path),
        _ => path.to_string(),
    }
}

pub fn parent_dir<P: AsRef<Path>>(path: P) -> (PathBuf, PathBuf) {
    let binding = path.as_ref();
    let filename = binding.file_name().expect("Path must have a filename");
    let parent = binding.parent().expect("Path must have a parent");
    (parent.to_path_buf(), filename.into())
}

// pub fn join_path<P: AsRef<Path>>(dir: P, name: P) -> String {
//     let mut input_dir: PathBuf = dir.as_ref().into();
//     input_dir.push(name);
//     input_dir.to_str().unwrap().to_string()
// }

pub fn input_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut filepath: PathBuf = root_dir().into();
    filepath.push(path);
    filepath
}

pub fn auto_create_dir_path<P: AsRef<Path>>(paths: Vec<P>) -> PathBuf {
    let mut filepath: PathBuf = root_dir().into();
    for path in paths {
        filepath.push(path);
    }

    let parent_dir = filepath.parent().unwrap();
    if !parent_dir.exists() {
        let _ = create_dir_all(&parent_dir);
    }

    filepath
}

pub fn buffer_path() -> PathBuf {
    get_cache_dir().join(BUFFER_FILE_NAME)
}

pub fn output_path<P: AsRef<Path>>(path: P) -> PathBuf {
    auto_create_dir_path(vec![&output_dir(), path.as_ref()])
}

pub fn hash_dir() -> PathBuf {
    get_cache_dir().join(HASH_DIR_NAME)
}

/// Return the hash file path `hash_dir/<path>.hash` for the given file or directory.
/// 
/// If the directory does not exist, it will be created.
pub fn hash_file_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut hash_path = hash_dir();
    hash_path.push(path);
    hash_path.set_extension(format!(
        "{}.hash",
        hash_path.extension().unwrap().to_str().unwrap()
    ));

    let parent_dir = hash_path.parent().unwrap();
    if !parent_dir.exists() {
        let _ = create_dir_all(&parent_dir);
    }
    hash_path
}

pub fn entry_dir() -> PathBuf {
    get_cache_dir().join(ENTRY_DIR_NAME)
}

/// Return the entry path `entry_dir/<path>` for the given file or directory.
///
/// If the directory does not exist, it will be created.
pub fn entry_path<P: AsRef<Path>>(path: P) -> PathBuf {
    auto_create_dir_path(vec![&entry_dir(), path.as_ref()]).into()
}

pub fn entry_file_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut entry_path = entry_dir();
    entry_path.push(path);
    entry_path.set_extension(format!(
        "{}.entry",
        entry_path.extension().unwrap().to_str().unwrap()
    ));
    entry_path
}

/// Return is file modified i.e. is hash updated.
pub fn is_hash_updated<P: AsRef<Path>>(content: &str, hash_path: P) -> (bool, u64) {
    let mut hasher = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(&content, &mut hasher);
    let current_hash = std::hash::Hasher::finish(&hasher);

    let history_hash = std::fs::read_to_string(&hash_path)
        .map(|s| s.parse::<u64>().expect("Invalid hash"))
        .unwrap_or(0); // no file: 0

    (current_hash != history_hash, current_hash)
}

/// Checks whether the file has been modified by comparing its current hash with the stored hash.
/// If the file is modified, updates the stored hash to reflect the latest state.
pub fn verify_and_file_hash<P: AsRef<Path>>(relative_path: P) -> eyre::Result<bool> {
    let root_dir = root_dir();
    let full_path = root_dir.join(&relative_path);
    let hash_path = hash_file_path(&relative_path);

    let content = std::fs::read_to_string(&full_path)
        .wrap_err_with(|| eyre::eyre!("Failed to read file `{}`", full_path.display()))?;
    let (is_modified, current_hash) = is_hash_updated(&content, &hash_path);
    if is_modified {
        std::fs::write(&hash_path, current_hash.to_string())
            .wrap_err_with(|| eyre::eyre!("Failed to write file `{}`", hash_path.display()))?;
    }
    return Ok(is_modified);
}

/// Checks whether the content has been modified by comparing its current hash with the stored hash.
/// If the content is modified, updates the stored hash to reflect the latest state.
pub fn verify_update_hash<P: AsRef<Path>>(path: P, content: &str) -> Result<bool, std::io::Error> {
    let hash_path = hash_file_path(path.as_ref());
    let (is_modified, current_hash) = is_hash_updated(&content, &hash_path);
    if is_modified {
        std::fs::write(&hash_path, current_hash.to_string())?;
    }

    Ok(is_modified)
}

#[allow(dead_code)]
pub fn delete_all_with<P: AsRef<Path>, F>(dir: P, predicate: &F) -> Result<(), std::io::Error>
where
    F: Fn(&Path) -> bool,
{
    for entry in WalkDir::new(dir) {
        let path = entry?.into_path();
        if path.is_file() && predicate(&path) {
            std::fs::remove_file(path)?;
        }
    }
    Ok(())
}
