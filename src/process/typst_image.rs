// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{fmt::Write, fs, path::PathBuf};

use crate::{
    config::{self, output_path, parent_dir},
    html_flake::{html_figure, html_figure_code},
    recorder::State,
    slug::Slug,
    typst_cli::{self, source_to_inline_html, write_svg, InlineConfig},
};
use pulldown_cmark::{Event, Tag, TagEnd};

use super::processer::url_action;

pub struct TypstImage<E> {
    events: E,
    state: State,
    shareds: Vec<String>,
    url: Option<String>,
    content: Option<String>,
    current_slug: Slug,
}

impl<E> TypstImage<E> {
    pub fn process(events: E, current_slug: Slug) -> Self {
        Self {
            events,
            state: State::None,
            shareds: Vec::new(),
            url: None,
            content: None,
            current_slug,
        }
    }

    fn exit(&mut self) {
        self.state = State::None;
        self.url = None;
        self.content = None;
    }
}

impl<'e, E: Iterator<Item = Event<'e>>> Iterator for TypstImage<E> {
    type Item = Event<'e>;

    fn next(&mut self) -> Option<Self::Item> {
        for e in self.events.by_ref() {
            match e {
                Event::Start(Tag::Link { ref dest_url, .. }) => {
                    let (url, action) = url_action(dest_url);
                    if is_inline_typst(dest_url) {
                        self.state = State::InlineTypst;
                        self.url = Some(dest_url.to_string()); // [0]
                    } else if action == State::ImageCode.strify() {
                        self.state = State::ImageCode;
                        self.url = Some(url.to_string());
                    } else if action == State::Html.strify() {
                        self.state = State::Html;
                        self.url = Some(url.to_string());
                    } else if action == State::Shared.strify() {
                        self.state = State::Shared;
                        self.url = Some(url.to_string());
                    } else if action == State::ImageBlock.strify() {
                        self.state = State::ImageBlock;
                        self.url = Some(url.to_string());
                    } else if action == State::ImageSpan.strify() {
                        self.state = State::ImageSpan;
                        self.url = Some(url.to_string());
                    } else {
                        return Some(e);
                    }
                }
                Event::Text(ref content) if allow_inline(&self.state) => {
                    self.content.get_or_insert_default().push_str(content);
                }
                Event::InlineMath(ref content) if allow_inline(&self.state) => {
                    let c = self.content.get_or_insert_default();
                    write!(c, "${content}$").unwrap();
                }
                Event::Code(ref content) if allow_inline(&self.state) => {
                    let c = self.content.get_or_insert_default();
                    write!(c, "<code>{content}</code>").unwrap();
                }
                Event::End(TagEnd::Link) => match self.state {
                    State::Html => {
                        let typst_url = config::relativize(&self.url.take().unwrap());
                        let (parent_dir, filename) = parent_dir(&typst_url);

                        let mut html_url = filename.with_extension("html");
                        let img_src = parent_dir.join(&html_url);
                        html_url = output_path(&img_src);

                        let html = match source_to_inline_html(PathBuf::from(typst_url), html_url) {
                            Ok(inline_html) => inline_html,
                            Err(err) => {
                                eprintln!("{:?} at {}", err, self.current_slug);
                                String::new()
                            }
                        };

                        self.exit();
                        return Some(Event::Html(html.into()));
                    }
                    State::InlineTypst => {
                        let shareds = self.shareds.join("\n");
                        let args: Vec<&str> = self.url.as_ref().unwrap().split("-").collect();
                        let mut args = &args[1..];
                        let mut auto_math_mode: bool = false;
                        if args.contains(&"math") {
                            auto_math_mode = true;
                            args = &args[1..];
                        }

                        let mut inline_typst = self.content.take().unwrap();
                        if auto_math_mode {
                            inline_typst = format!("${inline_typst}$");
                        }

                        let inline_typst = format!("{shareds}\n{inline_typst}");
                        let x = args.get(0);
                        let config = InlineConfig {
                            margin_x: x.map(|s| s.to_string()),
                            margin_y: args.get(1).or(x).map(|s| s.to_string()),
                        };
                        let html = match typst_cli::source_to_inline_svg(&inline_typst, config) {
                            Ok(svg) => svg,
                            Err(err) => {
                                eprintln!("{:?} at {}", err, self.current_slug);
                                String::new()
                            }
                        };

                        self.exit();
                        return Some(Event::Html(html.into()));
                    }
                    State::ImageSpan => {
                        let typst_url = self.url.as_ref().unwrap();
                        let caption = self.content.take().unwrap_or_default();
                        let typst_url = config::relativize(typst_url);
                        let (parent_dir, filename) = parent_dir(&typst_url);

                        let mut svg_url = filename.with_extension("svg");
                        let img_src = parent_dir.join(&svg_url);
                        svg_url = output_path(&img_src);

                        if let Err(err) = write_svg(PathBuf::from(typst_url), svg_url) {
                            eprintln!("{:?} at {}", err, self.current_slug)
                        }
                        self.exit();

                        let html = html_figure(&config::full_url(&img_src), false, caption);
                        return Some(Event::Html(html.into()));
                    }
                    State::ImageBlock => {
                        let typst_url = self.url.as_ref().unwrap();
                        let caption = self.content.take().unwrap_or_default();
                        let typst_url = config::relativize(typst_url);
                        let (parent_dir, filename) = parent_dir(&typst_url);

                        let mut svg_url = filename.with_extension("svg");
                        let img_src = parent_dir.join(&svg_url);
                        svg_url = output_path(&img_src);

                        if let Err(err) = write_svg(PathBuf::from(typst_url), svg_url) {
                            eprintln!("{:?} at {}", err, self.current_slug)
                        }
                        self.exit();

                        let html = html_figure(&config::full_url(&img_src), true, caption);
                        return Some(Event::Html(html.into()));
                    }
                    State::ImageCode => {
                        let typst_url = self.url.as_ref().unwrap();
                        let caption = self.content.take().unwrap_or_default();
                        let typst_url = config::relativize(typst_url);
                        let (parent_dir, filename) = parent_dir(&typst_url);

                        let mut svg_url = filename.with_extension("svg");
                        let img_src = parent_dir.join(&svg_url);
                        svg_url = output_path(&img_src);

                        if let Err(err) = write_svg(PathBuf::from(&typst_url), svg_url) {
                            eprintln!("{:?} at {}", err, self.current_slug)
                        }
                        self.exit();

                        let root_dir = config::root_dir();
                        let full_path = root_dir.join(typst_url);
                        let code = fs::read_to_string(format!("{}.code", full_path.display()))
                            .unwrap_or_else(|_| fs::read_to_string(full_path).unwrap());

                        let html = html_figure_code(&config::full_url(&img_src), caption, code);
                        return Some(Event::Html(html.into()));
                    }
                    State::Shared => {
                        let typst_url = self.url.take().unwrap();
                        let imported = self.content.take();
                        /*
                         * Unspecified import items will default to all (*),
                         * but we recommend users to manually enter "*" to avoid ambiguity.
                         */
                        let imported = imported.as_ref().map_or("*", |s| s);
                        self.shareds
                            .push(format!(r#"#import "{typst_url}": {imported}"#));

                        self.state = State::None;
                    }
                    _ => return Some(e),
                },
                _ => return Some(e),
            }
        }

        None
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
