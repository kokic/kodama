// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{fs, io::Write, path::Path, process::Command};

use crate::{
    config::{self, verify_and_file_hash},
    html_flake,
};

pub fn write_to_inline_html<P: AsRef<Path>>(typst_path: P, html_path: P) -> eyre::Result<String> {
    if !verify_and_file_hash(typst_path.as_ref())? && Path::new(html_path.as_ref()).exists() {
        let existed_html = fs::read_to_string(html_path)?;
        let existed_html = html_to_body_content(&existed_html);
        println!("Skip: {}", crate::slug::pretty_path(typst_path.as_ref()));
        return Ok(existed_html);
    }

    let root_dir = config::trees_dir();
    let html = to_html_string(typst_path.as_ref(), &root_dir)?;
    let html_body = html_to_body_content(&html);

    fs::write(&html_path, html)?;
    println!(
        "Compiled to HTML: {}",
        crate::slug::pretty_path(html_path.as_ref())
    );

    Ok(html_body)
}

pub fn html_to_body_content(html: &str) -> String {
    let start_pos = html.find("<html>").expect(concat!(file!(), '#', line!())) + 6;
    let end_pos = html.rfind("</html>").expect(concat!(file!(), '#', line!()));
    let content = &html[start_pos..end_pos];
    content.to_string()
}

pub struct InlineConfig {
    pub margin_x: Option<String>,
    pub margin_y: Option<String>,
}

impl InlineConfig {
    pub fn default_margin() -> String {
        "0em".to_string()
    }
}

pub fn source_to_inline_svg(src: &str, config: InlineConfig) -> eyre::Result<String> {
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

pub fn file_to_html(rel_path: &str, root_dir: &str) -> eyre::Result<String> {
    to_html_string(rel_path, root_dir).map(|s| html_to_body_content(&s))
}

fn to_html_string<P: AsRef<Path>>(rel_path: P, root_dir: P) -> eyre::Result<String> {
    let root_dir = root_dir.as_ref();
    let rel_path = rel_path.as_ref();
    let full_path = root_dir.join(rel_path);

    let output = Command::new("typst")
        .arg("c")
        .arg("-f=html")
        .arg("--features=html")
        .arg(format!("--root={}", root_dir.to_string_lossy()))
        .args(["--input", &format!("path={}", rel_path.to_string_lossy())])
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
        failed_in_file(
            concat!(file!(), '#', line!()),
            rel_path.to_str().unwrap(),
            stderr,
        );
        String::new()
    })
}

fn source_to_svg(src: &str) -> eyre::Result<String> {
    let root_dir = config::trees_dir();

    let mut typst = Command::new("typst")
        .arg("c")
        .arg("-f=svg")
        .arg(format!("--root={}", root_dir.to_string_lossy()))
        .arg("-")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    typst.stdin.take().unwrap().write_all(src.as_bytes())?;

    let output = typst.wait_with_output()?;
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

pub fn write_svg<P: AsRef<Path>>(typst_path: P, svg_path: P) -> eyre::Result<()> {
    let typst_path = typst_path.as_ref();
    let svg_path = svg_path.as_ref();

    if !verify_and_file_hash(typst_path)? && svg_path.exists() {
        println!("Skip: {}", crate::slug::pretty_path(typst_path));
        return Ok(());
    }

    let root_dir = config::trees_dir();
    let full_path = root_dir.join(typst_path);
    let output = Command::new("typst")
        .arg("c")
        .arg("-f=svg")
        .arg(format!("--root={}", root_dir.to_string_lossy()))
        .arg(&full_path)
        .arg(svg_path)
        .output()?;

    if output.status.success() {
        println!(
            "Compiled to SVG: {}",
            crate::slug::pretty_path(Path::new(svg_path))
        );
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        failed_in_file(
            concat!(file!(), '#', line!()),
            full_path.to_str().unwrap(),
            stderr,
        );
    }
    Ok(())
}

fn failed_in_file(src_pos: &'static str, file_path: &str, stderr: std::borrow::Cow<'_, str>) {
    eprintln!(
        "Command failed in {}: \n  In file {}, {}",
        src_pos, file_path, stderr
    );
}
