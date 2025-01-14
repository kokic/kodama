use std::process::Command;

use crate::{config::{self, verify_and_update_file_hash}, html};

pub fn write_svg(typst_path: &str, svg_path: &str) {
    if !verify_and_update_file_hash(typst_path) {
        println!(
            // "Skip compilation of unmodified: {}",
            "Skip: {}",
            crate::slug::pretty_path(std::path::Path::new(typst_path))
        );
        return;
    }

    let output = Command::new("typst")
        .arg("c")
        .arg(format!("--root={}", config::root_dir()))
        .arg(typst_path)
        .arg(svg_path)
        .output()
        .expect("Failed to execute command");

    if output.status.success() {
        let _ = String::from_utf8_lossy(&output.stdout);
        println!(
            "Compiled to SVG: {}",
            crate::slug::pretty_path(std::path::Path::new(svg_path))
        );
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Command failed in `write_svg`: \n  {}", stderr);
    }
}

pub struct InlineConfig {
    pub margin_x: Option<String>, 
    pub margin_y: Option<String>, 
    pub root_dir: String, 
}

impl InlineConfig {
    #[allow(dead_code)]
    pub fn new() -> InlineConfig {
        InlineConfig {
            margin_x: None,
            margin_y: None,
            root_dir: config::root_dir(),
        }
    }

    pub fn default_margin() -> String {
        "0em".to_string()
    }
}

pub fn source_to_inline_svg(src: &str, config: InlineConfig) -> String {
    let styles = format!(r#"
#set page(width: auto, height: auto, margin: (x: {}, y: {}), fill: rgb(0, 0, 0, 0)); 
#set text(size: 15.427pt, top-edge: "bounds", bottom-edge: "bounds");
    "#, config.margin_x.unwrap_or(InlineConfig::default_margin()), config.margin_y.unwrap_or(InlineConfig::default_margin()));
    let svg = source_to_svg(format!("{}{}", styles, src).as_str(), &config.root_dir);
    format!("\n{}\n", html!(span class = "inline-typst" => {svg}))
}

pub fn source_to_svg(src: &str, root_dir: &str) -> String {
    let output = if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args(["/C", &format!("echo {}", format!("'{}'", src))])
            .arg(format!("| typst c -f=svg --root={} - -", root_dir))
            .output()
            .expect("Failed to execute `echo` on Windows")
    } else {
        Command::new("sh")
            .args(["-c", &format!("echo {}", format!("'{}'", src))])
            .arg(format!("| typst c -f=svg --root={} - -", root_dir))
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
