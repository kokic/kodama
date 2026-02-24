// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{fs, io::Write, process::Command};

use camino::Utf8Path;
use eyre::{eyre, WrapErr};

use crate::{
    environment::{self, verify_and_file_hash},
    html_flake, path_utils,
};

pub fn write_to_inline_html<P: AsRef<Utf8Path>>(
    typst_path: P,
    html_path: P,
) -> eyre::Result<String> {
    if !verify_and_file_hash(typst_path.as_ref())? && html_path.as_ref().exists() {
        let existed_html = fs::read_to_string(html_path.as_ref())?;
        if let Ok(existed_html) = html_to_body_content(&existed_html) {
            if *crate::cli::build::verbose_skip() {
                println!("Skip: {}", path_utils::pretty_path(typst_path.as_ref()));
            }
            return Ok(existed_html);
        }
        color_print::ceprintln!(
            "<y>Warning: cached HTML `{}` is malformed, recompiling.</>",
            path_utils::pretty_path(html_path.as_ref())
        );
    }

    let root_dir = environment::trees_dir();
    let html = to_html_string(typst_path.as_ref(), &root_dir)?;
    let html_body = html_to_body_content(&html)?;

    fs::write(html_path.as_ref(), html)?;
    if *crate::cli::build::verbose() {
        println!(
            "Compiled to HTML: {}",
            path_utils::pretty_path(html_path.as_ref())
        );
    }
    Ok(html_body)
}

pub fn html_to_body_content(html: &str) -> eyre::Result<String> {
    let start_pos = html
        .find("<html>")
        .map(|pos| pos + 6)
        .ok_or_else(|| eyre!("missing `<html>` tag in typst html output"))?;
    let end_pos = html
        .rfind("</html>")
        .ok_or_else(|| eyre!("missing `</html>` tag in typst html output"))?;
    if end_pos < start_pos {
        return Err(eyre!("malformed html range in typst html output"));
    }
    let content = &html[start_pos..end_pos];
    Ok(content.to_string())
}

pub fn source_to_inline_svg(src: &str) -> eyre::Result<String> {
    let kodama_header = include_str!("include/kodama.typ");
    let svg = source_to_html(format!("{}\n#show: kodama\n{}", kodama_header, src).as_str())?;

    let start_pos = svg
        .find("<p>")
        .map(|pos| pos + 3)
        .ok_or_else(|| eyre!("missing `<p>` tag in typst inline svg output"))?;
    let end_pos = svg
        .rfind("</p>")
        .ok_or_else(|| eyre!("missing `</p>` tag in typst inline svg output"))?;
    if end_pos < start_pos {
        return Err(eyre!(
            "malformed paragraph range in typst inline svg output"
        ));
    }
    let content = &svg[start_pos..end_pos];

    Ok(format!(
        "\n{}\n",
        html_flake::html_inline_typst_span(content)
    ))
}

pub fn file_to_html(rel_path: &str, root_dir: &str) -> eyre::Result<String> {
    to_html_string(rel_path, root_dir)
        .and_then(|s| html_to_body_content(&s))
        .wrap_err_with(|| eyre!("failed to extract html body from `{}`", rel_path))
}

fn to_html_string<P: AsRef<Utf8Path>>(rel_path: P, root_dir: P) -> eyre::Result<String> {
    let root_dir = root_dir.as_ref();
    let rel_path = rel_path.as_ref();
    let full_path = root_dir.join(rel_path);
    if !full_path.exists() {
        return Err(eyre!(
            "typst source `{}` not found at `{}` (typst root: `{}`). Check `[build].typst-root` in \"Kodama.toml\"",
            rel_path,
            full_path,
            root_dir
        ));
    }

    let output = Command::new("typst")
        .arg("c")
        .arg("-f=html")
        .arg("--features=html")
        .arg(format!("--root={}", root_dir))
        .args(["--input", &format!("path={}", rel_path)])
        .args(["--input", &format!("random={}", fastrand::i64(0..))])
        .arg(full_path)
        .arg("-")
        .stdout(std::process::Stdio::piped())
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        failed_in_file(concat!(file!(), '#', line!()), rel_path.as_str(), stderr);
        Err(eyre!(
            "typst html compilation failed for `{}`",
            rel_path.as_str()
        ))
    }
}

fn source_to_html(src: &str) -> eyre::Result<String> {
    let root_dir = environment::trees_dir();

    let mut typst = Command::new("typst")
        .arg("c")
        .arg("-f=html")
        .arg("--features=html")
        .arg(format!("--root={}", root_dir))
        .arg("-")
        .arg("-")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    {
        let mut stdin = typst
            .stdin
            .take()
            .ok_or_else(|| eyre!("failed to open stdin for typst process"))?;
        stdin
            .write_all(src.as_bytes())
            .wrap_err("failed to write typst source to stdin")?;
    }

    let output = typst.wait_with_output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        color_print::ceprintln!(
            "<r>Command failed in {}: \n  {}</>",
            concat!(file!(), '#', line!()),
            stderr
        );
        Err(eyre!("typst inline html compilation failed"))
    }
}

pub fn write_svg<P: AsRef<Utf8Path>>(typst_path: P, svg_path: P) -> eyre::Result<()> {
    let typst_path = typst_path.as_ref();
    let svg_path = svg_path.as_ref();

    if !verify_and_file_hash(typst_path)? && svg_path.exists() {
        if *crate::cli::build::verbose_skip() {
            println!("Skip: {}", path_utils::pretty_path(typst_path));
        }
        return Ok(());
    }

    let trees_dir = environment::trees_dir();
    let full_path = trees_dir.join(typst_path);
    let output = Command::new("typst")
        .arg("c")
        .arg("-f=svg")
        .arg(format!("--root={}", trees_dir))
        .arg(&full_path)
        .arg(svg_path)
        .output()?;

    if output.status.success() {
        if *crate::cli::build::verbose() {
            println!(
                "Compiled to SVG: {}",
                path_utils::pretty_path(Utf8Path::new(svg_path))
            );
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        failed_in_file(concat!(file!(), '#', line!()), full_path.as_str(), stderr);
    }
    Ok(())
}

fn failed_in_file(src_pos: &'static str, file_path: &str, stderr: std::borrow::Cow<'_, str>) {
    color_print::ceprintln!(
        "<r>Command failed in {}: \n  In file {}, {}</>",
        src_pos,
        file_path,
        stderr
    );
}

#[cfg(test)]
mod tests {
    use super::{html_to_body_content, to_html_string};
    use camino::{Utf8Path, Utf8PathBuf};
    use std::fs;

    #[test]
    fn test_html_to_body_content_ok() {
        let html = "<html><p>Hello</p></html>";
        let body = html_to_body_content(html).unwrap();
        assert_eq!(body, "<p>Hello</p>");
    }

    #[test]
    fn test_html_to_body_content_missing_tags() {
        let err = html_to_body_content("<p>Hello</p>");
        assert!(err.is_err());
    }

    #[test]
    fn test_to_html_string_reports_typst_root_hint_for_missing_source() {
        let base =
            std::env::temp_dir().join(format!("kodama-typst-root-hint-{}", fastrand::u64(..)));
        fs::create_dir_all(&base).unwrap();
        let base = Utf8PathBuf::from_path_buf(base).unwrap();

        let err = to_html_string(
            Utf8Path::new("references/feature-typst.typst"),
            base.as_path(),
        )
        .unwrap_err();
        let message = format!("{err:#}");
        assert!(message.contains("Check `[build].typst-root`"));
        assert!(message.contains("references/feature-typst.typst"));

        let _ = fs::remove_dir_all(base.as_std_path());
    }
}
