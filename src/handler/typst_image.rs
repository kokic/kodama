use super::{url_action, Handler};
use crate::{
    adjust_name,
    config::{self, join_path, output_path, parent_dir},
    html_flake::html_figure,
    kodama::parse_spanned_markdown,
    recorder::{Recorder, State},
    typst_cli::{self, write_svg, InlineConfig},
};
use pulldown_cmark::{Tag, TagEnd};

pub struct TypstImage;

impl Handler for TypstImage {
    fn start(&mut self, tag: &Tag<'_>, recorder: &mut Recorder) {
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
                } else if action == State::ImageBlock.strify() {
                    recorder.enter(State::ImageBlock);
                    recorder.push(url.to_string());
                } else if action == State::ImageSpan.strify() {
                    recorder.enter(State::ImageSpan);
                    recorder.push(url.to_string());
                } else if action == State::Shared.strify() {
                    recorder.enter(State::Shared);
                    recorder.push(url.to_string());
                }
            }
            _ => (),
        }
    }

    fn end(&mut self, tag: &TagEnd, recorder: &mut Recorder, history: &mut Vec<String>) -> Option<String> {
        if tag == &TagEnd::Link {
            match recorder.state {
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
                    let s = match typst_cli::source_to_inline_svg(&inline_typst, config) {
                        Ok(svg) => svg,
                        Err(err) => {
                            eprintln!("{:?} at {}", err, recorder.current);
                            String::new()
                        }
                    };
                    recorder.exit();
                    return Some(s);
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

                    let current = recorder.current.to_string();
                    let caption = match parse_spanned_markdown(&caption, current, history) {
                        Ok(html) => html,
                        Err(err) => {
                            eprintln!("{:?}", err);
                            caption
                        }
                    };
                    return Some(html_figure(&config::full_url(&img_src), false, caption));
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

                    let current = recorder.current.to_string();
                    let caption = match parse_spanned_markdown(&caption, current, history) {
                        Ok(html) => html,
                        Err(err) => {
                            eprintln!("{:?}", err);
                            caption
                        }
                    };
                    return Some(html_figure(&config::full_url(&img_src), true, caption));
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
        recorder: &mut Recorder,
        _metadata: &mut std::collections::HashMap<String, String>,
        _history: &mut Vec<String>
    ) {
        if recorder.state == State::Shared
            || recorder.state == State::InlineTypst
            || recorder.state == State::ImageSpan
            || recorder.state == State::ImageBlock
        {
            // [1]: imported / inline typst / span / block
            return recorder.push(s.to_string());
        }
    }
}

pub fn is_inline_typst(dest_url: &str) -> bool {
    let key = State::InlineTypst.strify();
    dest_url == key || dest_url.starts_with(&format!("{}-", key))
}
