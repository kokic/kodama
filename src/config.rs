use std::{fs::create_dir_all, path::PathBuf, sync::Mutex};

pub static ROOT_DIR: Mutex<String> = Mutex::new(String::new());
pub static OUTPUT_DIR: Mutex<String> = Mutex::new(String::new());

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
