use std::{
    hash::{Hash, Hasher},
    os::windows::process::CommandExt,
    process::Command,
};

use crate::{config::hash_path, html};

pub fn is_typst_modified(typst_path: &str) -> bool {
    let hash_path = hash_path(typst_path);
    let src = std::fs::read_to_string(typst_path).unwrap();

    let mut hasher = std::hash::DefaultHasher::new();
    src.hash(&mut hasher);
    let current_hash = hasher.finish();
    
    let history_hash = std::fs::read_to_string(&hash_path)
        .map(|s| s.parse::<u64>().expect("Invalid hash"))
        .unwrap_or(0); // no file: 0

    let is_modified = current_hash != history_hash;
    if is_modified {
        let _ = std::fs::write(&hash_path, current_hash.to_string());
    }
    is_modified
}

pub fn write_svg(typst_path: &str, svg_path: &str) {
    if !is_typst_modified(typst_path) {
        println!("Skip compilation of unmodified: {}", typst_path);
        return;
    }

    let output = Command::new("typst")
        .arg("c")
        .arg(typst_path)
        .arg(svg_path)
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        let _ = String::from_utf8_lossy(&output.stdout);
        println!("Compiled to SVG: {}", svg_path);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Command failed in `write_svg`: \n  {}", stderr);
    }
}

pub fn source_to_inline_svg(src: &str) -> String {
    let styles = r#"
#set page(width: auto, height: auto, margin: (x: 0em, y: 0em), fill: rgb(0, 0, 0, 0)); 
#set text(size: 15.427pt, top-edge: "bounds", bottom-edge: "bounds");
    "#;
    let svg = source_to_svg(format!("{}{}", styles, src).as_str());
    format!("\n{}\n", html!(span class = "inline-typst" => {svg}))
}

pub fn source_to_svg(src: &str) -> String {
    let output = if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args(["/C", &format!("echo {}", format!("'{}'", src))])
            .raw_arg("| typst c -f=svg - -")
            .output()
            .expect("Failed to execute `echo` on Windows")
    } else {
        Command::new("sh")
            .args(["-c", &format!("echo {}", format!("'{}'", src))])
            .raw_arg("| typst c -f=svg - -")
            .output()
            .expect("Failed to execute `echo`")
    };

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.to_string()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Command failed with error:\n{}", stderr);
        String::new()
    }
}
