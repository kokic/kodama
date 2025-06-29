// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{fs, path::Path, process::Command};

use crate::{
    config::{self, verify_and_file_hash},
    html_flake,
};

pub fn source_to_inline_html<P: AsRef<Path>>(typst_path: P, html_path: P) -> eyre::Result<String> {
    if !verify_and_file_hash(typst_path.as_ref())? && Path::new(html_path.as_ref()).exists() {
        let existed_html = fs::read_to_string(html_path)?;
        let existed_html = html_to_body_content(&existed_html);
        println!("Skip: {}", crate::slug::pretty_path(typst_path.as_ref()));
        return Ok(existed_html);
    }

    let root_dir = config::root_dir();
    let full_path = root_dir.join(typst_path);
    let html = source_to_html(&full_path, &root_dir)?;
    let html_body = html_to_body_content(&html);

    fs::write(&html_path, html)?;
    println!(
        "Compiled to HTML: {}",
        crate::slug::pretty_path(&html_path.as_ref())
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
}

impl InlineConfig {
    #[allow(dead_code)]
    pub fn new() -> InlineConfig {
        InlineConfig {
            margin_x: None,
            margin_y: None,
        }
    }

    pub fn default_margin() -> String {
        "0em".to_string()
    }
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
    let svg = source_to_svg(format!("{}{}", styles, src).as_str())?;

    Ok(format!("\n{}\n", html_flake::html_inline_typst_span(&svg)))
}

pub fn source_to_html<P: AsRef<Path>>(full_path: P, root_dir: P) -> Result<String, std::io::Error> {

    let full_path = full_path.as_ref();
    let output = Command::new("typst")
        .arg("c")
        .arg("-f=html")
        .arg(format!("--root={}", root_dir.as_ref().display()))
        .arg("--features=html")
        .arg(&full_path)
        .arg("-")
        .stdout(std::process::Stdio::piped())
        .output()?;

    Ok(if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.to_string()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        failed_in_file(concat!(file!(), '#', line!()), &full_path.to_str().unwrap(), stderr);
        String::new()
    })
}

pub fn source_to_svg(src: &str) -> Result<String, std::io::Error> {
    compile_source(src, "svg")
}

/// inline typst to inline svg (deprecated)
pub fn compile_source(
    src: &str,
    output_format: &str,
) -> Result<String, std::io::Error> {
    let buffer_path = config::buffer_path();
    fs::write(&buffer_path, src)?;

    let output = Command::new("typst")
        .arg("c")
        .arg(format!("-f={}", output_format))
        .arg(format!("--root={}", crate::config::root_dir().display()))
        .arg(&buffer_path)
        .arg("-")
        .stdout(std::process::Stdio::piped())
        .output()?;
    fs::remove_file(buffer_path)?;

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

pub fn file_to_html(typst_path: &str, root_dir: &str) -> Result<String, std::io::Error> {
    compile_file(typst_path, root_dir, "html", Some("--features=html"))
        .map(|s| html_to_body_content(&s))
}

pub fn compile_file(
    typst_path: &str,
    root_dir: &str,
    output_format: &str,
    extra: Option<&str>,
) -> Result<String, std::io::Error> {
    let output = Command::new("typst")
        .arg("c")
        .arg(format!("-f={}", output_format))
        .arg(format!("--root={}", root_dir))
        .args(extra)
        .arg(typst_path.to_string())
        .arg("-")
        .stdout(std::process::Stdio::piped())
        .output()?;

    Ok(if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.to_string()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        failed_in_file(concat!(file!(), '#', line!()), &typst_path, stderr);
        String::new()
    })
}

/// typst file to svg (`stdout -> disk`)
pub fn write_svg<P: AsRef<Path>>(typst_path: P, svg_path: P) -> eyre::Result<()> {
    
    let typst_path = typst_path.as_ref();
    let svg_path = svg_path.as_ref();

    if !verify_and_file_hash(typst_path)? && svg_path.exists() {
        println!("Skip: {}", crate::slug::pretty_path(typst_path));
        return Ok(());
    }

    let root_dir = config::root_dir();
    let full_path = root_dir.join(typst_path);
    let output = Command::new("typst")
        .arg("c")
        .arg("-f=svg")
        .arg(format!("--root={}", root_dir.display()))
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
        failed_in_file(concat!(file!(), '#', line!()), &full_path.to_str().unwrap(), stderr);
    }
    Ok(())
}

fn failed_in_file(src_pos: &'static str, file_path: &str, stderr: std::borrow::Cow<'_, str>) {
    eprintln!(
        "Command failed in {}: \n  In file {}, {}",
        src_pos, file_path, stderr
    );
}
