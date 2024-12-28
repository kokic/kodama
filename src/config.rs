use std::{fs::create_dir_all, path::PathBuf, sync::Mutex};

pub static ROOT_DIR: Mutex<String> = Mutex::new(String::new());
pub static OUTPUT_DIR: Mutex<String> = Mutex::new(String::new());

pub const CACHE_DIR: &str = "./.cache";

pub fn dir_config(source: &Mutex<String>, target: String) {
    let mut path = source.lock().unwrap();
    *path = target;
}

pub fn root_dir() -> String {
    ROOT_DIR.lock().unwrap().to_string()
}

pub fn parent_dir(path: &str) -> (String, String) {
    let binding = PathBuf::from(path);
    let filename = binding.file_name().unwrap().to_str().unwrap();
    let parent = binding.parent().unwrap().to_str().unwrap();
    (parent.to_string(), filename.to_string())
}

#[allow(dead_code)]
pub fn parent_dir_create_all(path: &str) -> (String, String) {
    let binding = PathBuf::from(path);
    let filename = binding.file_name().unwrap().to_str().unwrap();
    
    let parent = binding.parent().unwrap();
    if !parent.exists() {
        let _ = create_dir_all(&parent);
    }
    
    (parent.to_str().unwrap().to_string(), filename.to_string())
}

pub fn join_path(dir: &str, name: &str) -> String {
    let mut input_dir: PathBuf = dir.into();
    input_dir.push(name);
    input_dir.to_str().unwrap().to_string()
}

pub fn input_path(path: &str) -> String {
    let mut filepath: PathBuf = root_dir().into();
    filepath.push(path);
    filepath.to_str().unwrap().to_string()
}

pub fn output_path(path: &str) -> String {
    let mut filepath: PathBuf = root_dir().into();
    filepath.push(OUTPUT_DIR.lock().unwrap().to_string());
    filepath.push(path);

    let parent_dir = filepath.parent().unwrap();
    if !parent_dir.exists() {
        let _ = create_dir_all(&parent_dir);
    }

    filepath.to_str().unwrap().to_string()
}

pub fn cache_path(path: &str) -> PathBuf {
    let mut filepath: PathBuf = root_dir().into();
    filepath.push(CACHE_DIR);
    filepath.push(path);
    
    let parent_dir = filepath.parent().unwrap();
    if !parent_dir.exists() {
        let _ = create_dir_all(&parent_dir);
    }

    filepath
}

pub fn is_file_modified(path: &str) -> bool {
    let hash_path = cache_path(&format!("{}.hash", path));
    let src = std::fs::read_to_string(path).unwrap();

    let mut hasher = std::hash::DefaultHasher::new();
    std::hash::Hash::hash(&src, &mut hasher);
    let current_hash = std::hash::Hasher::finish(&hasher);
    
    let history_hash = std::fs::read_to_string(&hash_path)
        .map(|s| s.parse::<u64>().expect("Invalid hash"))
        .unwrap_or(0); // no file: 0

    let is_modified = current_hash != history_hash;
    if is_modified {
        let _ = std::fs::write(&hash_path, current_hash.to_string());
    }
    is_modified
}
