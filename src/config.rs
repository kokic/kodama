// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{
    fs::{self, create_dir_all},
    path::{Path, PathBuf},
    str::FromStr,
    sync::{LazyLock, OnceLock},
};

use eyre::Context;
use serde::{Deserialize, Serialize};

use crate::{config_toml::Config, slug::Slug};

#[derive(Debug, Clone, clap::ValueEnum, Default, Deserialize, Serialize)]
pub enum FooterMode {
    #[default]
    #[serde(rename = "link")]
    Link,

    #[serde(rename = "embed")]
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

impl std::fmt::Display for FooterMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FooterMode::Link => write!(f, "link"),
            FooterMode::Embed => write!(f, "embed"),
        }
    }
}

/// Specifies the filename of the TOML configuration file (e.g., "Kodama.toml").
pub static TOML: OnceLock<String> = OnceLock::new();

/// Specifies the project root path.
///
/// Please note that this value should always be automatically derived from
/// the location of the toml configuration file.
pub static ROOT: OnceLock<PathBuf> = OnceLock::new();

#[derive(Clone)]
pub enum BuildMode {
    /// Build mode for the `kodama build` command.
    Build,

    /// Serve mode for the `kodama serve` command.
    Serve,
}

pub static BUILD_MODE: OnceLock<BuildMode> = OnceLock::new();

pub static CONFIG_TOML: OnceLock<Config> = OnceLock::new();

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

pub fn root_dir() -> PathBuf {
    ROOT.get().unwrap().clone()
}

pub fn is_serve() -> bool {
    matches!(BUILD_MODE.get().unwrap(), BuildMode::Serve)
}

pub fn is_short_slug() -> bool {
    CONFIG_TOML.get().unwrap().build.short_slug
}

pub fn typst_root_dir() -> PathBuf {
    CONFIG_TOML.get().unwrap().build.typst_root.clone().into()
}

pub fn trees_dir() -> PathBuf {
    let trees = &CONFIG_TOML.get().unwrap().kodama.trees;
    root_dir().join(trees)
}

pub fn output_dir() -> PathBuf {
    let output_dir = match BUILD_MODE.get().unwrap() {
        BuildMode::Build => &CONFIG_TOML.get().unwrap().build.output,
        BuildMode::Serve => &CONFIG_TOML.get().unwrap().serve.output,
    };
    root_dir().join(output_dir)
}

pub fn base_url() -> String {
    CONFIG_TOML.get().unwrap().kodama.base_url.clone()
}

pub fn footer_mode() -> FooterMode {
    CONFIG_TOML.get().unwrap().build.footer_mode.clone()
}

pub fn inline_css() -> bool {
    CONFIG_TOML.get().unwrap().build.inline_css
}

pub fn editor_url() -> Option<String> {
    CONFIG_TOML.get().unwrap().serve.edit.clone()
}

pub fn get_cache_dir() -> PathBuf {
    root_dir().join(CACHE_DIR_NAME)
}

pub fn assets_dir() -> PathBuf {
    let assets = &CONFIG_TOML.get().unwrap().kodama.assets;
    root_dir().join(assets)
}

/// URL keep posix style, so the type of return value is [`String`].
pub fn full_url<P: AsRef<Path>>(path: P) -> String {
    let path = crate::slug::pretty_path(path.as_ref());
    if let Some(stripped) = path.strip_prefix("/") {
        return format!("{}{}", base_url(), stripped);
    } else if let Some(stripped) = path.strip_prefix("./") {
        return format!("{}{}", base_url(), stripped);
    }
    format!("{}{}", base_url(), path)
}

pub fn full_html_url(slug: Slug) -> String {
    let pretty_urls = CONFIG_TOML.get().unwrap().build.pretty_urls;
    let page_suffix = to_page_suffix(pretty_urls);
    full_url(format!("{}{}", slug, page_suffix))
}

pub fn parent_dir<P: AsRef<Path>>(path: P) -> (PathBuf, PathBuf) {
    let binding = path.as_ref();
    let filename = binding.file_name().expect("Path must have a filename");
    let parent = binding.parent().expect("Path must have a parent");
    (parent.to_path_buf(), filename.into())
}

pub fn input_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut filepath: PathBuf = trees_dir();
    filepath.push(path);
    filepath
}

pub fn create_parent_dirs<P: AsRef<Path>>(path: P) {
    let parent_dir = path.as_ref().parent().unwrap();
    if !parent_dir.exists() {
        let _ = create_dir_all(parent_dir);
    }
}

pub fn auto_create_dir_path<P: AsRef<Path>>(paths: Vec<P>) -> PathBuf {
    let mut filepath: PathBuf = root_dir();
    for path in paths {
        filepath.push(path);
    }
    create_parent_dirs(&filepath);
    filepath
}

pub fn output_path<P: AsRef<Path>>(path: P) -> PathBuf {
    auto_create_dir_path(vec![&output_dir(), path.as_ref()])
}

/// Return the output HTML path `<output_dir>/<path>.html` for the given section.
/// e.g. `/path/to/index.md` will return `<output_dir>/path/to/index.html`.
///
/// If the directory does not exist, it will be created.
#[allow(dead_code)]
pub fn output_html_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut output_path = output_dir();
    output_path.push(path);
    output_path.set_extension("html");
    create_parent_dirs(&output_path);
    output_path
}

pub fn hash_dir() -> PathBuf {
    get_cache_dir().join(HASH_DIR_NAME)
}

/// Return the hash file path `<hash_dir>/<path>.hash` for the given file or directory.
/// e.g. `/path/to/index.md` will return `<hash_dir>/path/to/index.md.hash`.
///
/// If the directory does not exist, it will be created.
pub fn hash_file_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut hash_path = hash_dir();
    hash_path.push(path);
    hash_path.set_extension(format!(
        "{}.hash",
        hash_path.extension().unwrap().to_str().unwrap()
    ));
    create_parent_dirs(&hash_path);
    hash_path
}

pub fn entry_dir() -> PathBuf {
    get_cache_dir().join(ENTRY_DIR_NAME)
}

/// Return the hash file path `<hash_dir>/<path>.hash` for the given file or directory.
/// e.g. `/path/to/index.md` will return `<entry_dir>/path/to/index.md.entry`.
///
/// If the directory does not exist, it will be created.
pub fn entry_file_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let mut entry_path = entry_dir();
    entry_path.push(path);
    entry_path.set_extension(format!(
        "{}.entry",
        entry_path.extension().unwrap().to_str().unwrap()
    ));
    create_parent_dirs(&entry_path);
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
    let root_dir = trees_dir();
    let full_path = root_dir.join(&relative_path);
    let hash_path = hash_file_path(&relative_path);

    let content = std::fs::read_to_string(&full_path)
        .wrap_err_with(|| eyre::eyre!("failed to read file `{}`", full_path.display()))?;
    let (is_modified, current_hash) = is_hash_updated(&content, &hash_path);
    if is_modified {
        std::fs::write(&hash_path, current_hash.to_string())
            .wrap_err_with(|| eyre::eyre!("failed to write file `{}`", hash_path.display()))?;
    }
    Ok(is_modified)
}

/// Checks whether the content has been modified by comparing its current hash with the stored hash.
/// If the content is modified, updates the stored hash to reflect the latest state.
pub fn verify_update_hash<P: AsRef<Path>>(path: P, content: &str) -> Result<bool, std::io::Error> {
    let hash_path = hash_file_path(path.as_ref());
    let (is_modified, current_hash) = is_hash_updated(content, &hash_path);
    if is_modified {
        std::fs::write(&hash_path, current_hash.to_string())?;
    }

    Ok(is_modified)
}
