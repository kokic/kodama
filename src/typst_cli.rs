use std::{os::windows::process::CommandExt, process::Command};

use crate::html;

pub fn write_svg(input: &str, svg_path: &str) {
    let output = Command::new("typst")
        .arg("c")
        .arg(input)
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

pub fn to_inline_svg(input: &str) -> String {
    let styles = r#"
#set page(width: auto, height: auto, margin: (x: 0em, y: 0em), fill: rgb(0, 0, 0, 0)); 
#set text(size: 15.427pt, top-edge: "bounds", bottom-edge: "bounds");
    "#;
    let svg = to_svg(format!("{}{}", styles, input).as_str());
    format!("\n{}\n", html!(span class = "inline-typst" => {svg}))
}

pub fn to_svg(input: &str) -> String {
    let output = if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args(["/C", &format!("echo {}", format!("'{}'", input))])
            .raw_arg("| typst c -f=svg - -")
            .output()
            .expect("Failed to execute `echo` on Windows")
    } else {
        Command::new("sh")
            .args(["-c", &format!("echo {}", format!("'{}'", input))])
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
