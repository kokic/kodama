use super::{url_action, Handler};
use crate::{
    adjust_name,
    config::{join_path, output_path, parent_dir},
    html_flake::{html_center_image, html_image},
    recorder::{Context, Recorder},
    typst_cli::{self, write_svg},
};
use pulldown_cmark::{Tag, TagEnd};

pub struct TypstImage {}

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
                if dest_url.to_string() == Context::InlineTypst.strify() {
                    recorder.enter(Context::InlineTypst);
                } else if action == Context::ImageBlock.strify() {
                    recorder.enter(Context::ImageBlock);
                    recorder.push(url.to_string());
                } else if action == Context::ImageSpan.strify() {
                    recorder.enter(Context::ImageSpan);
                    recorder.push(url.to_string());
                }
            }
            _ => {}
        }
    }

    fn end(&mut self, tag: &TagEnd, recorder: &mut Recorder) -> Option<String> {
        if tag == &TagEnd::Link {
            let relative_dir = &recorder.relative_dir;
            match recorder.context {
                Context::InlineTypst => {
                    let inline_typst = recorder.data.get(0).unwrap().as_str();
                    let s = typst_cli::source_to_inline_svg(inline_typst);
                    recorder.exit();
                    return Some(s);
                }
                Context::ImageSpan => {
                    let typst_url = recorder.data.get(0).unwrap().as_str();
                    let typst_url = join_path(relative_dir, typst_url);
                    let (parent_dir, filename) = parent_dir(&typst_url);

                    let mut svg_url = adjust_name(&filename, ".typ", ".svg");
                    let img_src = join_path(&parent_dir, &svg_url);
                    svg_url = output_path(&img_src);

                    write_svg(&typst_url, &svg_url);
                    recorder.exit();
                    return Some(html_image(&img_src));
                }
                Context::ImageBlock => {
                    let typst_url = recorder.data.get(0).unwrap().as_str();
                    let typst_url = join_path(relative_dir, typst_url);
                    let (parent_dir, filename) = parent_dir(&typst_url);

                    let mut svg_url = adjust_name(&filename, ".typ", ".svg");
                    let img_src = join_path(&parent_dir, &svg_url);
                    svg_url = output_path(&img_src);

                    write_svg(&typst_url, &svg_url);
                    recorder.exit();
                    return Some(html_center_image(&img_src));
                }

                _ => (),
            }
        }
        None
    }
}
