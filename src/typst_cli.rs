use std::{fs, path::Path, process::Command};

use crate::{
    config::{self, verify_and_file_hash},
    html, html_flake,
};

pub fn write_html(typst_path: &str, html_path: &str) -> Result<(), std::io::Error> {
    if !verify_and_file_hash(typst_path)? && Path::new(html_path).exists() {
        println!("Skip: {}", crate::slug::pretty_path(Path::new(typst_path)));
        return Ok(());
    }

    let root_dir = config::root_dir();
    let full_path = config::join_path(&root_dir, typst_path);
    let content = fs::read_to_string(full_path).expect(concat!(file!(), '#', line!()));
    let content = export_html_config(&content);

    let html = source_to_html(&content, &root_dir)?;
    fs::write(html_path, html)?;
    println!(
        "Compiled to HTML: {}",
        crate::slug::pretty_path(Path::new(html_path))
    );

    Ok(())
}

fn export_html_config(content: &str) -> String {
    format!(
        r#"
#show: html.elem.with("html")
#let elem-frame(content, attrs: (:), tag: "div") = html.elem(tag, html.frame(content), attrs: attrs)
#show math.equation.where(block: true): elem-frame.with(tag: "span", attrs: ("style": "display: flex; justify-content: center; overflow: auto;"))
#show math.equation.where(block: false): elem-frame.with(tag: "span", attrs: ("style": "display: inline;"))
    {}
    "#,
        content
    )
}

pub fn read_typst_html_body(html_path: &str) -> Result<String, std::io::Error> {
    let html = fs::read_to_string(html_path)?;
    let start_pos = html.find("<html>").expect(concat!(file!(), '#', line!())) + 6;
    let end_pos = html.rfind("</html>").expect(concat!(file!(), '#', line!()));
    let content = &html[start_pos..end_pos];
    return Ok(content.to_string());
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

    Ok(format!(
        "\n{}\n",
        html!(span class = "inline-typst" => {svg})
    ))
}

pub fn source_to_html(src: &str, root_dir: &str) -> Result<String, std::io::Error> {
    compile_source(src, root_dir, "html", Some("--features=html"))
}

pub fn source_to_svg(src: &str, root_dir: &str) -> Result<String, std::io::Error> {
    compile_source(src, root_dir, "svg", None)
}

pub fn compile_source(
    src: &str,
    root_dir: &str,
    output_format: &str,
    extra: Option<&str>,
) -> Result<String, std::io::Error> {
    let buffer_path = config::buffer_path();
    fs::write(&buffer_path, src)?;

    let extra_and_buffer = extra.map_or(vec![buffer_path.to_string()], |s| {
        vec![s.to_string(), buffer_path.to_string()]
    });
    let output = Command::new("typst")
        .arg("c")
        .arg(format!("-f={}", output_format))
        .arg(format!("--root={}", root_dir))
        .args(extra_and_buffer)
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
        .arg(full_path)
        .arg("-")
        .stdout(std::process::Stdio::piped())
        .output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let thematized = thematize(stdout);
        fs::write(svg_path, thematized)?;

        println!(
            "Compiled to SVG: {}",
            crate::slug::pretty_path(Path::new(svg_path))
        );
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!(
            "Command failed in {}: \n  {}",
            concat!(file!(), '#', line!()),
            stderr
        );
    }
    Ok(())
}

fn thematize(s: std::borrow::Cow<'_, str>) -> String {
    let index = s.rfind("</svg>").unwrap();
    format!(
        "{}<style>\n{}\n</style>\n</svg>",
        &s[0..index],
        html_flake::html_typst_style()
    )
}
