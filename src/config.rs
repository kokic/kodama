use std::{
    fs::{self, create_dir_all},
    hash::Hash,
    path::{Path, PathBuf},
    sync::{LazyLock, Mutex},
};

use walkdir::WalkDir;

use crate::slug::Slug;

#[derive(Clone, clap::ValueEnum)]
pub enum FooterMode {
    Link,
    Embed,
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
    pub base_url: S,
    pub page_suffix: S,
    pub short_slug: bool,
    pub footer_mode: FooterMode,

    /// `false`: This is very useful for users who want to modify existing styles or configure other themes.
    pub disable_export_css: bool,
}

impl CompileConfig<&'static str> {
    pub const fn default() -> CompileConfig<&'static str> {
        CompileConfig {
            root_dir: "./",
            output_dir: "./publish",
            base_url: "/",
            page_suffix: "",
            short_slug: true,
            footer_mode: FooterMode::Link,
            disable_export_css: true,
        }
    }
}

impl CompileConfig<String> {
    const fn empty() -> CompileConfig<String> {
        CompileConfig {
            root_dir: String::new(),
            output_dir: String::new(),
            base_url: String::new(),
            page_suffix: String::new(),
            short_slug: true,
            footer_mode: FooterMode::Link,
            disable_export_css: true,
        }
    }

    pub fn new<'a>(
        root_dir: String,
        output_dir: String,
        base_url: String,
        disable_pretty_urls: bool,
        short_slug: bool,
        footer_mode: FooterMode,
        disable_export_css: bool,
    ) -> CompileConfig<String> {
        CompileConfig {
            root_dir,
            output_dir,
            base_url: normalize_base_url(&base_url),
            page_suffix: to_page_suffix(disable_pretty_urls),
            short_slug,
            footer_mode,
            disable_export_css,
        }
    }
}

pub static DEFAULT_CONFIG: CompileConfig<&'static str> = CompileConfig::default();
pub static CONFIG: Mutex<CompileConfig<String>> = Mutex::new(CompileConfig::empty());

pub static CUSTOM_META_HTML: LazyLock<String> = LazyLock::new(|| {
    std::fs::read_to_string(join_path(&root_dir(), "import-meta.html")).unwrap_or_default()
});

pub static CUSTOM_FONTS_HTML: LazyLock<String> = LazyLock::new(|| {
    fs::read_to_string(join_path(&root_dir(), "import-fonts.html"))
        .unwrap_or(include_str!("include/import-fonts.html").to_string())
});

pub static CUSTOM_MATH_HTML: LazyLock<String> = LazyLock::new(|| {
    fs::read_to_string(join_path(&root_dir(), "import-math.html"))
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

pub fn root_dir() -> String {
    lock_config().root_dir.to_string()
}

pub fn output_dir() -> String {
    lock_config().output_dir.to_string()
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

pub fn get_cache_dir() -> String {
    join_path(&root_dir(), CACHE_DIR_NAME)
}

pub fn full_url(path: &str) -> String {
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

/**
 * `path` to `./{path}` or `path`.
 */
pub fn relativize(path: &str) -> String {
    match path.starts_with("/") {
        true => format!(".{}", path),
        _ => path.to_string(),
    }
}

pub fn parent_dir(path: &str) -> (String, String) {
    let binding = PathBuf::from(path);
    let filename = binding.file_name().unwrap().to_str().unwrap();
    let parent = binding.parent().unwrap().to_str().unwrap();
    (parent.to_string(), filename.to_string())
}

pub fn join_path(dir: &str, name: &str) -> String {
    let mut input_dir: PathBuf = dir.into();
    input_dir.push(name);
    input_dir.to_str().unwrap().to_string().replace("\\", "/")
}

pub fn input_path<P: AsRef<Path>>(path: P) -> String {
    let mut filepath: PathBuf = root_dir().into();
    filepath.push(path);
    filepath.to_str().unwrap().to_string()
}

pub fn auto_create_dir_path(paths: Vec<&str>) -> String {
    let mut filepath: PathBuf = root_dir().into();
    for path in paths {
        filepath.push(path);
    }

    let parent_dir = filepath.parent().unwrap();
    if !parent_dir.exists() {
        let _ = create_dir_all(&parent_dir);
    }

    filepath.to_str().unwrap().to_string()
}

pub fn buffer_path() -> String {
    join_path(&get_cache_dir(), BUFFER_FILE_NAME)
}

pub fn output_path(path: &str) -> String {
    auto_create_dir_path(vec![&output_dir(), path])
}

pub fn hash_dir() -> String {
    join_path(&get_cache_dir(), HASH_DIR_NAME)
}

pub fn hash_path(path: &str) -> PathBuf {
    auto_create_dir_path(vec![&hash_dir(), path]).into()
}

pub fn entry_dir() -> String {
    join_path(&get_cache_dir(), ENTRY_DIR_NAME)
}

pub fn entry_path(path: &str) -> PathBuf {
    auto_create_dir_path(vec![&entry_dir(), path]).into()
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
pub fn verify_and_file_hash(relative_path: &str) -> Result<bool, std::io::Error> {
    let root_dir = root_dir();
    let full_path = join_path(&root_dir, relative_path);
    let hash_path = hash_path(&format!("{}.hash", relative_path));

    let content = std::fs::read_to_string(full_path)?;
    let (is_modified, current_hash) = is_hash_updated(&content, &hash_path);
    if is_modified {
        std::fs::write(&hash_path, current_hash.to_string())?;
    }
    return Ok(is_modified);
}

/// Checks whether the content has been modified by comparing its current hash with the stored hash.
/// If the content is modified, updates the stored hash to reflect the latest state.
pub fn verify_update_hash(path: &str, content: &str) -> Result<bool, std::io::Error> {
    let hash_path = hash_path(&format!("{}.hash", path));
    let (is_modified, current_hash) = is_hash_updated(&content, &hash_path);
    if is_modified {
        std::fs::write(&hash_path, current_hash.to_string())?;
    }

    Ok(is_modified)
}

#[allow(dead_code)]
pub fn delete_all_with<F>(dir: &str, predicate: &F) -> Result<(), std::io::Error>
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

#[allow(dead_code)]
pub fn delete_all_built_files() -> Result<(), std::io::Error> {
    let root_dir = root_dir();
    std::fs::remove_dir_all(join_path(&root_dir, &get_cache_dir()))?;
    std::fs::remove_dir_all(join_path(&root_dir, &output_dir()))?;
    Ok(())
}
