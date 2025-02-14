use std::{fs, path::Path, process::Command};

use crate::{
    config::{self, verify_and_file_hash},
    html, html_flake,
};

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
        eprintln!("Command failed in `write_svg`: \n  {}", stderr);
    }
    Ok(())
}

fn thematize(s: std::borrow::Cow<'_, str>) -> String {
    let index = s.rfind("</svg>").unwrap();
    format!("{}<style>\n{}\n</style>\n</svg>", &s[0..index], html_flake::html_typst_style())
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

pub fn source_to_svg(src: &str, root_dir: &str) -> Result<String, std::io::Error> {
    let child = if cfg!(target_os = "windows") {
        Command::new("powershell")
            .arg("/C")
            .arg(format!(
                "echo '{}' | typst c -f=svg --root={} - -",
                src, root_dir
            ))
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn shell process")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(format!(
                "echo '{}' | typst c -f=svg --root={} - -",
                src, root_dir
            ))
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn shell process")
    };
    let output = child.wait_with_output().expect("Failed to read stdout");

    Ok(if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout.to_string()
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Command failed with error:\n{}", stderr);
        String::new()
    })
}
