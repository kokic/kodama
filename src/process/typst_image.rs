use std::fs;

use crate::{
    compiler::section::LazyContent,
    config::{self, join_path, output_path, parent_dir},
    html_flake::{html_figure, html_figure_code},
    recorder::{ParseRecorder, State},
    slug::adjust_name,
    typst_cli::{self, write_html, write_svg, InlineConfig},
};
use pulldown_cmark::{Tag, TagEnd};

use super::processer::{url_action, Processer};

pub struct TypstImage;

impl Processer for TypstImage {
    fn start(&mut self, tag: &Tag<'_>, recorder: &mut ParseRecorder) {
        match tag {
            Tag::Link {
                link_type: _,
                dest_url,
                title: _,
                id: _,
            } => {
                let (url, action) = url_action(dest_url);
                if is_inline_typst(dest_url) {
                    recorder.enter(State::InlineTypst);
                    recorder.push(dest_url.to_string()); // [0]
                } else if action == State::ImageCode.strify() {
                    recorder.enter(State::ImageCode);
                    recorder.push(url.to_string());
                } else if action == State::Html.strify() {
                    recorder.enter(State::Html);
                    recorder.push(url.to_string());
                } else if action == State::Shared.strify() {
                    recorder.enter(State::Shared);
                    recorder.push(url.to_string());
                } else if action == State::ImageBlock.strify() {
                    recorder.enter(State::ImageBlock);
                    recorder.push(url.to_string());
                } else if action == State::ImageSpan.strify() {
                    recorder.enter(State::ImageSpan);
                    recorder.push(url.to_string());
                }
            }
            _ => (),
        }
    }

    fn end(&mut self, tag: &TagEnd, recorder: &mut ParseRecorder) -> Option<LazyContent> {
        if tag == &TagEnd::Link {
            match recorder.state {
                State::Html => {
                    let typst_url = recorder.data.get(0).unwrap().as_str();
                    let typst_url = config::relativize(typst_url);
                    let (parent_dir, filename) = parent_dir(&typst_url);

                    let mut html_url = adjust_name(&filename, ".typ", ".html");
                    let img_src = join_path(&parent_dir, &html_url);
                    html_url = output_path(&img_src);

                    match write_html(&typst_url, &html_url) {
                        Err(err) => eprintln!("{:?} at {}", err, recorder.current),
                        Ok(_) => (),
                    }
                    recorder.exit();

                    let html = typst_cli::read_typst_html_body(&html_url).unwrap_or_default();
                    return Some(LazyContent::Plain(html));
                }
                State::InlineTypst => {
                    let shareds = recorder.shareds.join("\n");
                    let args: Vec<&str> = recorder.data.get(0).unwrap().split("-").collect();
                    let mut args = &args[1..];
                    let mut auto_math_mode: bool = false;
                    if args.contains(&"math") {
                        auto_math_mode = true;
                        args = &args[1..];
                    }

                    let mut inline_typst = recorder.data.get(1).unwrap().to_string();
                    if auto_math_mode {
                        inline_typst = format!("${}$", inline_typst);
                    }

                    let inline_typst = format!("{}\n{}", shareds, inline_typst);
                    let x = args.get(0);
                    let config = InlineConfig {
                        margin_x: x.map(|s| s.to_string()),
                        margin_y: args.get(1).or(x).map(|s| s.to_string()),
                        root_dir: config::root_dir(),
                    };
                    let html = match typst_cli::source_to_inline_svg(&inline_typst, config) {
                        Ok(svg) => svg,
                        Err(err) => {
                            eprintln!("{:?} at {}", err, recorder.current);
                            String::new()
                        }
                    };

                    recorder.exit();
                    return Some(LazyContent::Plain(html));
                }
                State::ImageSpan => {
                    let typst_url = recorder.data.get(0).unwrap().as_str();
                    let caption = match recorder.data.len() > 1 {
                        true => recorder.data[1..].join(""),
                        false => String::new(),
                    };
                    let typst_url = config::relativize(typst_url);
                    let (parent_dir, filename) = parent_dir(&typst_url);

                    let mut svg_url = adjust_name(&filename, ".typ", ".svg");
                    let img_src = join_path(&parent_dir, &svg_url);
                    svg_url = output_path(&img_src);

                    match write_svg(&typst_url, &svg_url) {
                        Err(err) => eprintln!("{:?} at {}", err, recorder.current),
                        Ok(_) => (),
                    }
                    recorder.exit();

                    let html = html_figure(&config::full_url(&img_src), false, caption);
                    return Some(LazyContent::Plain(html));
                }
                State::ImageBlock => {
                    let typst_url = recorder.data.get(0).unwrap().as_str();
                    let caption = match recorder.data.len() > 1 {
                        true => recorder.data[1..].join(""),
                        false => String::new(),
                    };
                    let typst_url = config::relativize(typst_url);
                    let (parent_dir, filename) = parent_dir(&typst_url);

                    let mut svg_url = adjust_name(&filename, ".typ", ".svg");
                    let img_src = join_path(&parent_dir, &svg_url);
                    svg_url = output_path(&img_src);

                    match write_svg(&typst_url, &svg_url) {
                        Err(err) => eprintln!("{:?} at {}", err, recorder.current),
                        Ok(_) => (),
                    }
                    recorder.exit();

                    let html = html_figure(&config::full_url(&img_src), true, caption);
                    return Some(LazyContent::Plain(html));
                }
                State::ImageCode => {
                    let typst_url = recorder.data.get(0).unwrap().as_str();
                    let caption = match recorder.data.len() > 1 {
                        true => recorder.data[1..].join(""),
                        false => String::new(),
                    };
                    let typst_url = config::relativize(typst_url);
                    let (parent_dir, filename) = parent_dir(&typst_url);

                    let mut svg_url = adjust_name(&filename, ".typ", ".svg");
                    let img_src = join_path(&parent_dir, &svg_url);
                    svg_url = output_path(&img_src);

                    match write_svg(&typst_url, &svg_url) {
                        Err(err) => eprintln!("{:?} at {}", err, recorder.current),
                        Ok(_) => (),
                    }
                    recorder.exit();

                    let root_dir = config::root_dir();
                    let full_path = config::join_path(&root_dir, &typst_url);
                    let code = fs::read_to_string(format!("{}.code", full_path))
                        .unwrap_or_else(|_| fs::read_to_string(full_path).unwrap());

                    let html = html_figure_code(&config::full_url(&img_src), caption, code);
                    return Some(LazyContent::Plain(html));
                }
                State::Shared => {
                    let typst_url = recorder.data.get(0).unwrap().as_str();
                    let imported = recorder.data.get(1);
                    let imported = match imported {
                        Some(s) => s,
                        /*
                         * Unspecified import items will default to all (*),
                         * but we recommend users to manually enter "*" to avoid ambiguity.
                         */
                        None => "*",
                    };
                    recorder
                        .shareds
                        .push(format!(r#"#import "{}": {}"#, typst_url, imported));
                    recorder.exit();
                }

                _ => (),
            }
        }
        None
    }

    fn text(
        &self,
        s: &pulldown_cmark::CowStr<'_>,
        recorder: &mut ParseRecorder,
        _metadata: &mut std::collections::HashMap<String, String>,
    ) {
        if allow_inline(&recorder.state) {
            // [1]: imported / inline typst / span / block
            return recorder.push(s.to_string());
        }
    }

    fn inline_math(
        &self,
        s: &pulldown_cmark::CowStr<'_>,
        recorder: &mut ParseRecorder,
    ) -> Option<std::string::String> {
        if allow_inline(&recorder.state) {
            recorder.push(format!("${}$", s)); // [1, 2, ...]: Text
        }
        None
    }

    fn code(&self, s: &pulldown_cmark::CowStr<'_>, recorder: &mut ParseRecorder) {
        if allow_inline(&recorder.state) {
            recorder.push(format!("<code>{}</code>", s));
        }
    }
}

fn allow_inline(state: &State) -> bool {
    *state == State::Shared
        || *state == State::InlineTypst
        || *state == State::Html
        || *state == State::ImageSpan
        || *state == State::ImageBlock
        || *state == State::ImageCode
}

pub fn is_inline_typst(dest_url: &str) -> bool {
    let key = State::InlineTypst.strify();
    dest_url == key || dest_url.starts_with(&format!("{}-", key))
}
