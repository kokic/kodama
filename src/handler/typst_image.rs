use super::{url_action, Handler};
use crate::{
    adjust_name,
    config::{self, join_path, output_path, parent_dir},
    html_flake::{html_center_image, html_image},
    recorder::{Context, Recorder},
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
                    recorder.enter(Context::InlineTypst);
                    recorder.push(dest_url.to_string()); // [0]
                } else if action == Context::ImageBlock.strify() {
                    recorder.enter(Context::ImageBlock);
                    recorder.push(url.to_string());
                } else if action == Context::ImageSpan.strify() {
                    recorder.enter(Context::ImageSpan);
                    recorder.push(url.to_string());
                } else if action == Context::Shared.strify() {
                    recorder.enter(Context::Shared);
                    recorder.push(url.to_string());
                }
            }
            _ => ()
        }
    }

    fn end(&mut self, tag: &TagEnd, recorder: &mut Recorder) -> Option<String> {
        if tag == &TagEnd::Link {
            match recorder.context {
                Context::InlineTypst => {
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
                    let s = typst_cli::source_to_inline_svg(&inline_typst, config);
                    recorder.exit();
                    return Some(s);
                }
                Context::ImageSpan => {
                    let typst_url = recorder.data.get(0).unwrap().as_str();
                    let typst_url = config::relativize(typst_url);
                    let (parent_dir, filename) = parent_dir(&typst_url);

                    let mut svg_url = adjust_name(&filename, ".typ", ".svg");
                    let img_src = join_path(&parent_dir, &svg_url);
                    svg_url = output_path(&img_src);

                    write_svg(&typst_url, &svg_url);
                    recorder.exit();
                    return Some(html_image(&config::full_url(&img_src)));
                }
                Context::ImageBlock => {
                    let typst_url = recorder.data.get(0).unwrap().as_str();
                    let typst_url = config::relativize(typst_url);
                    let (parent_dir, filename) = parent_dir(&typst_url);

                    let mut svg_url = adjust_name(&filename, ".typ", ".svg");
                    let img_src = join_path(&parent_dir, &svg_url);
                    svg_url = output_path(&img_src);

                    write_svg(&typst_url, &svg_url);
                    recorder.exit();
                    return Some(html_center_image(&config::full_url(&img_src)));
                }
                Context::Shared => {
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
    ) {
        if recorder.context == Context::Shared || recorder.context == Context::InlineTypst {
            return recorder.push(s.to_string()); // [1]: imported / inline typst
        }
    }
}

pub fn is_inline_typst(dest_url: &str) -> bool {
    let key = Context::InlineTypst.strify();
    dest_url == key || dest_url.starts_with(&format!("{}-", key))
}
