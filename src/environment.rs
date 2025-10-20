// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{
    fs::{self, create_dir_all},
    sync::{LazyLock, OnceLock},
};

use camino::{Utf8Path, Utf8PathBuf};
use eyre::Context;

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
    pub config: Config,
    pub build_mode: BuildMode,
}

static ENVIRONMENT: OnceLock<Environment> = OnceLock::new();

fn get_environment() -> &'static Environment {
    ENVIRONMENT.get().expect("environment must be initialized")
}

fn get_config() -> &'static Config {
    &get_environment().config
}

pub fn init_environment(toml_file: Utf8PathBuf, build_mode: BuildMode) -> eyre::Result<()> {
    let toml_file = config::find_config(toml_file)?;

    let (root, _file_name) = path_utils::split_file_name(&toml_file).expect("path cannot be empty");
    let toml = std::fs::read_to_string(&toml_file)?;

    _ = ENVIRONMENT.set(Environment {
        root: root.to_owned(),
        config: config::parse_config(&toml)?,
        build_mode,
    });
    Ok(())
}

/// Mock environment for testing purposes.
#[allow(dead_code)]
pub fn mock_environment() -> eyre::Result<()> {
    _ = ENVIRONMENT.set(Environment {
        root: "./".into(),
        config: Config::default(),
        build_mode: BuildMode::Build,
    });
    Ok(())
}

#[derive(Clone)]
pub enum BuildMode {
    /// Build mode for the `kodama build` command.
    Build,

    /// Serve mode for the `kodama serve` command.
    Serve,
}

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

pub fn root_dir() -> &'static Utf8Path {
    &get_environment().root
}

pub fn is_serve() -> bool {
    matches!(get_environment().build_mode, BuildMode::Serve)
}

pub fn is_build() -> bool {
    matches!(get_environment().build_mode, BuildMode::Build)
}

pub fn is_short_slug() -> bool {
    get_config().build.short_slug
}

pub fn typst_root_dir() -> &'static Utf8Path {
    Utf8Path::new(&get_config().build.typst_root)
}

pub fn trees_dir() -> Utf8PathBuf {
    let trees = &get_environment().config.kodama.trees;
    root_dir().join(trees)
}

pub fn output_dir() -> Utf8PathBuf {
    let output_dir = match get_environment().build_mode {
        BuildMode::Build => &get_config().build.output,
        BuildMode::Serve => &get_config().serve.output,
    };
    root_dir().join(output_dir)
}

pub fn base_url() -> &'static str {
    let env = get_environment();
    match env.build_mode {
        BuildMode::Build => &env.config.kodama.base_url,
        BuildMode::Serve => kodama::DEFAULT_BASE_URL,
    }
}

pub fn is_toc_left() -> bool {
    match get_config().toc.placement {
        toc::TocPlacement::Left => true,
        toc::TocPlacement::Right => false,
    }
}

pub fn is_toc_sticky() -> bool {
    get_config().toc.sticky
}

pub fn is_toc_mobile_sticky() -> bool {
    get_config().toc.mobile_sticky
}

pub fn toc_max_width() -> String {
    get_config().toc.max_width.clone()
}

pub fn get_edit_text() -> String {
    get_config().text.edit.clone()
}

pub fn get_toc_text() -> String {
    get_config().text.toc.clone()
}

pub fn get_footer_references_text() -> String {
    get_config().text.references.clone()
}

pub fn get_footer_backlinks_text() -> String {
    get_config().text.backlinks.clone()
}

pub fn footer_mode() -> FooterMode {
    get_config().build.footer_mode
}

pub fn inline_css() -> bool {
    get_config().build.inline_css
}

pub fn asref() -> bool {
    get_config().build.asref
}

pub fn deploy_edit_url() -> Option<&'static str> {
    get_config().build.edit.as_deref()
}

pub fn editor_url() -> Option<&'static str> {
    get_config().serve.edit.as_deref()
}

pub fn serve_command() -> Vec<String> {
    get_config().serve.command.clone()
}

pub fn get_cache_dir() -> Utf8PathBuf {
    root_dir().join(CACHE_DIR_NAME)
}

pub fn assets_dir() -> Utf8PathBuf {
    let assets = &get_config().kodama.assets;
    root_dir().join(assets)
}

/// URL keep posix style, so the type of return value is [`String`].
pub fn full_url<P: AsRef<Utf8Path>>(path: P) -> String {
    let base_url = base_url();
    let path = path_utils::pretty_path(path.as_ref());
    if let Some(stripped) = path.strip_prefix("/") {
        return format!("{}{}", base_url, stripped);
    } else if let Some(stripped) = path.strip_prefix("./") {
        return format!("{}{}", base_url, stripped);
    }
    format!("{}{}", base_url, path)
}

pub fn full_html_url(slug: Slug) -> String {
    let pretty_urls = get_config().build.pretty_urls;
    let page_suffix = to_page_suffix(pretty_urls);
    full_url(format!("{}{}", slug, page_suffix))
}

pub fn input_path<P: AsRef<Utf8Path>>(path: P) -> Utf8PathBuf {
    let mut filepath: Utf8PathBuf = trees_dir();
    filepath.push(path);
    filepath
}

pub fn create_parent_dirs<P: AsRef<Utf8Path>>(path: P) {
    let parent_dir = path.as_ref().parent().unwrap();
    if !parent_dir.exists() {
        let _ = create_dir_all(parent_dir);
    }
}

pub fn auto_create_dir_path<P: AsRef<Utf8Path>>(paths: Vec<P>) -> Utf8PathBuf {
    let mut filepath: Utf8PathBuf = root_dir().to_owned();
    for path in paths {
        filepath.push(path);
    }
    create_parent_dirs(&filepath);
    filepath
}

pub fn output_path<P: AsRef<Utf8Path>>(path: P) -> Utf8PathBuf {
    auto_create_dir_path(vec![&output_dir(), path.as_ref()])
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
    hash_path.set_extension(format!("{}.hash", hash_path.extension().unwrap()));
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
    entry_path.set_extension(format!("{}.entry", entry_path.extension().unwrap()));
    create_parent_dirs(&entry_path);
    entry_path
}

/// Return is file modified i.e. is hash updated.
pub fn is_hash_updated<P: AsRef<Utf8Path>>(content: &str, hash_path: P) -> (bool, u64) {
    let mut hasher = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(&content, &mut hasher);
    let current_hash = std::hash::Hasher::finish(&hasher);

    let history_hash = std::fs::read_to_string(hash_path.as_ref())
        .map(|s| s.parse::<u64>().expect("Invalid hash"))
        .unwrap_or(0); // no file: 0

    (current_hash != history_hash, current_hash)
}

/// Checks whether the file has been modified by comparing its current hash with the stored hash.
/// If the file is modified, updates the stored hash to reflect the latest state.
pub fn verify_and_file_hash<P: AsRef<Utf8Path>>(relative_path: P) -> eyre::Result<bool> {
    if crate::environment::is_build() {
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
    if crate::environment::is_build() {
        return Ok(true);
    }

    let hash_path = hash_file_path(path.as_ref());
    let (is_modified, current_hash) = is_hash_updated(content, &hash_path);
    if is_modified {
        std::fs::write(&hash_path, current_hash.to_string())?;
    }

    Ok(is_modified)
}
