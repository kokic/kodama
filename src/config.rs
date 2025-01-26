use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
    sync::Mutex,
};

pub static ROOT_DIR: Mutex<String> = Mutex::new(String::new());
pub static OUTPUT_DIR: Mutex<String> = Mutex::new(String::new());
pub static BASE_URL: Mutex<String> = Mutex::new(String::new());
pub static PAGE_SUFFIX: Mutex<String> = Mutex::new(String::new());

/// compiled & written markdown URLs
pub static HISTORY: Mutex<Vec<String>> = Mutex::new(vec![]);

pub fn history() -> Vec<String> {
    let history = HISTORY.lock().unwrap();
    history.to_vec()
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Blink {
    pub source: String,
    pub target: String,
}

impl Blink {
    pub fn new(source: String, target: String) -> Self {
        return Blink { source, target };
    }
}

/// linked markdown URLs
pub static LINKED: Mutex<Vec<Blink>> = Mutex::new(vec![]);

pub fn linked() -> Vec<Blink> {
    let linked = LINKED.lock().unwrap();
    linked.to_vec()
}

pub const CACHE_DIR: &str = "./.cache";
pub const HASH_DIR_NAME: &str = "hash";
// pub const ENTRY_DIR_NAME: &str = "entry";

pub fn dir_config(source: &Mutex<String>, target: String) {
    let mut path = source.lock().unwrap();
    *path = target;
}

pub fn root_dir() -> String {
    ROOT_DIR.lock().unwrap().to_string()
}

pub fn base_url() -> String {
    BASE_URL.lock().unwrap().to_string()
}

pub fn page_suffix() -> String {
    PAGE_SUFFIX.lock().unwrap().to_string()
}

pub fn full_url(path: &str) -> String {
    if path.starts_with("/") {
        return format!("{}{}", base_url(), path[1..].to_string());
    } else if path.starts_with("./") {
        return format!("{}{}", base_url(), path[2..].to_string());
    }
    format!("{}{}", base_url(), path)
}

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

pub fn output_path(path: &str) -> String {
    auto_create_dir_path(vec![&OUTPUT_DIR.lock().unwrap(), path])
}

pub fn hash_dir() -> String {
    join_path(CACHE_DIR, HASH_DIR_NAME)
}

pub fn hash_path(path: &str) -> PathBuf {
    auto_create_dir_path(vec![&hash_dir(), path]).into()
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
pub fn verify_and_update_file_hash(path: &str) -> bool {
    let hash_path = hash_path(&format!("{}.hash", path));
    let content = std::fs::read_to_string(path);
    if let Ok(content) = content {
        let (is_modified, current_hash) = is_hash_updated(&content, &hash_path);
        if is_modified {
            let _ = std::fs::write(&hash_path, current_hash.to_string());
        }
        return is_modified;
    }
    true
}

/// Checks whether the content has been modified by comparing its current hash with the stored hash.
/// If the content is modified, updates the stored hash to reflect the latest state.
pub fn verify_and_update_content_hash(path: &str, content: &str) -> bool {
    let hash_path = hash_path(&format!("{}.hash", path));
    let (is_modified, current_hash) = is_hash_updated(&content, &hash_path);
    if is_modified {
        let _ = std::fs::write(&hash_path, current_hash.to_string());
    }
    is_modified
}

pub fn delete_all_build_files() -> Result<(), std::io::Error> {
    let root_dir = root_dir();
    std::fs::remove_dir_all(join_path(&root_dir, CACHE_DIR))?;
    std::fs::remove_dir_all(join_path(&root_dir, &OUTPUT_DIR.lock().unwrap()))?;
    Ok(())
}
