use std::process::Command;

use crate::{config::verify_and_update_file_hash, html};

pub fn write_svg(typst_path: &str, svg_path: &str) {
    if !verify_and_update_file_hash(typst_path) {
        println!(
            "Skip compilation of unmodified: {}",
            crate::slug::pretty_path(std::path::Path::new(typst_path))
        );
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
        println!(
            "Compiled to SVG: {}",
            crate::slug::pretty_path(std::path::Path::new(svg_path))
        );
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
            .arg("| typst c -f=svg - -")
            .output()
            .expect("Failed to execute `echo` on Windows")
    } else {
        Command::new("sh")
            .args(["-c", &format!("echo {}", format!("'{}'", src))])
            .arg("| typst c -f=svg - -")
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
