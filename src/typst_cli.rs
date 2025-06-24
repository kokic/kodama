// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{fs, path::Path, process::Command};

use crate::{
    config::{self, verify_and_file_hash},
    html_flake,
};

pub fn source_to_inline_html(typst_path: &str, html_path: &str) -> Result<String, std::io::Error> {
    if !verify_and_file_hash(typst_path)? && Path::new(html_path).exists() {
        let existed_html = fs::read_to_string(html_path)?;
        let existed_html = html_to_body_content(&existed_html);
        println!("Skip: {}", crate::slug::pretty_path(Path::new(typst_path)));
        return Ok(existed_html);
    }

    let root_dir = config::root_dir();
    let html = source_to_html(&typst_path, &root_dir)?;
    let html_body = html_to_body_content(&html);

    fs::write(html_path, html)?;
    println!(
        "Compiled to HTML: {}",
        crate::slug::pretty_path(Path::new(html_path))
    );

    Ok(html_body)
}

pub fn html_to_body_content(html: &str) -> String {
    let start_pos = html.find("<html>").expect(concat!(file!(), '#', line!())) + 6;
    let end_pos = html.rfind("</html>").expect(concat!(file!(), '#', line!()));
    let content = &html[start_pos..end_pos];
    return content.to_string();
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

pub fn file_to_html(rel_path: &str, root_dir: &str) -> Result<String, std::io::Error> {
    source_to_html(rel_path, root_dir).map(|s| html_to_body_content(&s))
}

pub fn source_to_inline_svg(src: &str, config: InlineConfig) -> Result<String, std::io::Error> {
    let styles = format!(
        r#"
#set page(width: auto, height: auto, margin: (x: {}, y: {}), fill: rgb(0, 0, 0, 0)); 
#set text(size: 15.427pt, top-edge: "bounds", bottom-edge: "bounds");
    "#,
        config.margin_x.unwrap_or(InlineConfig::default_margin()),
        config.margin_y.unwrap_or(InlineConfig::default_margin())
    );
    let svg = source_to_svg(format!("{}{}", styles, src).as_str(), &config.root_dir)?;

    Ok(format!("\n{}\n", html_flake::html_inline_typst_span(&svg)))
}

fn source_to_html(rel_path: &str, root_dir: &str) -> Result<String, std::io::Error> {
    let full_path = config::join_path(&root_dir, rel_path);

    let output = Command::new("typst")
        .arg("c")
        .arg("-f=html")
        .arg("--features=html")
        .arg(format!("--root={}", root_dir))
        .args(["--input", &format!("path={}", rel_path)])
        .args(["--input", &format!("sha256={}", sha256::digest(&full_path))]) // sha256 of the path!
        .args(["--input", &format!("random={}", fastrand::i64(0..))])
        .arg(full_path)
        .arg("-")
        .stdout(std::process::Stdio::piped())
        .output()?;

    Ok(if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.to_string()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        failed_in_file(concat!(file!(), '#', line!()), rel_path, stderr);
        String::new()
    })
}

fn source_to_svg(src: &str, root_dir: &str) -> Result<String, std::io::Error> {
    struct Buffer {
        path: String,
    }

    impl Drop for Buffer {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    let root_dir = root_dir;
    let buffer = Buffer {
        path: config::buffer_path(),
    };
    fs::write(&buffer.path, src)?;

    let output = Command::new("typst")
        .arg("c")
        .arg("-f=svg")
        .arg(format!("--root={}", root_dir))
        .arg(&buffer.path)
        .arg("-")
        .stdout(std::process::Stdio::piped())
        .output()?;

    Ok(if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.to_string()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!(
            "Command failed in {}: \n  {}",
            concat!(file!(), '#', line!()),
            stderr
        );
        String::new()
    })
}

/// typst file to svg (`stdout -> disk`)
pub fn write_svg(typst_path: &str, svg_path: &str) -> Result<(), std::io::Error> {
    if !verify_and_file_hash(typst_path)? && Path::new(svg_path).exists() {
        println!("Skip: {}", crate::slug::pretty_path(Path::new(typst_path)));
        return Ok(());
    }

    let root_dir = config::root_dir();
    let full_path = config::join_path(&root_dir, typst_path);
    let output = Command::new("typst")
        .arg("c")
        .arg("-f=svg")
        .arg(format!("--root={}", root_dir))
        .arg(&full_path)
        .arg("-")
        .stdout(std::process::Stdio::piped())
        .output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        fs::write(svg_path, stdout.as_bytes())?;

        println!(
            "Compiled to SVG: {}",
            crate::slug::pretty_path(Path::new(svg_path))
        );
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        failed_in_file(concat!(file!(), '#', line!()), &full_path, stderr);
    }
    Ok(())
}

fn failed_in_file(src_pos: &'static str, file_path: &str, stderr: std::borrow::Cow<'_, str>) {
    eprintln!(
        "Command failed in {}: \n  In file {}, {}",
        src_pos, file_path, stderr
    );
}
