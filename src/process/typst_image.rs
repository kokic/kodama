// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{
    fmt::Write,
    fs,
    sync::atomic::{AtomicBool, Ordering},
};

use camino::Utf8PathBuf;
use pulldown_cmark::{Event, Tag, TagEnd};

use crate::{
    environment::{self, output_path},
    html_flake::{html_figure, html_figure_code},
    recorder::State,
    slug::Slug,
    typst_cli::{self, write_to_inline_html},
};

use super::{
    path_resolution::{relocate_trees_path, resolve_section_url},
    processer::url_action,
};

static TYPEST_IMAGE_ERROR_FLAG: AtomicBool = AtomicBool::new(false);

pub fn reset_typst_image_error_flag() {
    TYPEST_IMAGE_ERROR_FLAG.store(false, Ordering::Relaxed);
}

pub fn typst_image_error_detected() -> bool {
    TYPEST_IMAGE_ERROR_FLAG.load(Ordering::Relaxed)
}

fn record_typst_image_error() {
    TYPEST_IMAGE_ERROR_FLAG.store(true, Ordering::Relaxed);
}

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
                    let _ = write!(c, "${content}$");
                }
                Event::Code(ref content) if allow_inline(&self.state) => {
                    let c = self.content.get_or_insert_default();
                    let _ = write!(c, "<code>{content}</code>");
                }
                Event::End(TagEnd::Link) => match self.state {
                    State::Html => {
                        let typst_url =
                            typst_path(self.current_slug, &self.url.take().unwrap_or_default());
                        let html = if environment::is_check() {
                            let trees_dir = environment::trees_dir();
                            match typst_cli::file_to_html(typst_url.as_str(), trees_dir.as_str()) {
                                Ok(inline_html) => inline_html,
                                Err(err) => {
                                    record_typst_image_error();
                                    color_print::ceprintln!(
                                        "<r>{:?} at {}</>",
                                        err,
                                        self.current_slug
                                    );
                                    String::new()
                                }
                            }
                        } else {
                            let html_path = output_path(typst_url.with_extension("html"));
                            match write_to_inline_html(typst_url, html_path) {
                                Ok(inline_html) => inline_html,
                                Err(err) => {
                                    record_typst_image_error();
                                    color_print::ceprintln!(
                                        "<r>{:?} at {}</>",
                                        err,
                                        self.current_slug
                                    );
                                    String::new()
                                }
                            }
                        };

                        self.exit();
                        return Some(Event::Html(html.into()));
                    }
                    State::InlineTypst => {
                        let shareds = self.shareds.join("\n");
                        let inline_url = if let Some(url) = self.url.take() {
                            url
                        } else {
                            color_print::ceprintln!(
                                "<y>Warning: missing inline typst url at `{}`.</>",
                                self.current_slug
                            );
                            self.state = State::None;
                            self.content = None;
                            continue;
                        };
                        let args: Vec<&str> = inline_url.split("-").collect();
                        let args = &args[1..];
                        let mut auto_math_mode: bool = false;
                        if args.contains(&"math") {
                            auto_math_mode = true;
                        }

                        let mut inline_typst = self.content.take().unwrap_or_default();
                        inline_typst = smart_punctuation_reverse(&inline_typst);

                        if auto_math_mode {
                            inline_typst = format!("${}$", inline_typst);
                        }

                        let inline_typst = format!("{shareds}\n{inline_typst}");
                        let html = match typst_cli::source_to_inline_svg(&inline_typst) {
                            Ok(svg) => svg,
                            Err(err) => {
                                record_typst_image_error();
                                color_print::ceprintln!("<r>{:?} at {}</>", err, self.current_slug);
                                String::new()
                            }
                        };

                        self.exit();
                        return Some(Event::Html(html.into()));
                    }
                    State::ImageSpan => {
                        let typst_url =
                            typst_path(self.current_slug, &self.url.take().unwrap_or_default());
                        let caption = self.content.take().unwrap_or_default();
                        let svg_url = typst_url.with_extension("svg");
                        self.exit();

                        let html = html_figure(&environment::full_url(&svg_url), false, caption);
                        return Some(Event::Html(html.into()));
                    }
                    State::ImageBlock => {
                        let typst_url =
                            typst_path(self.current_slug, &self.url.take().unwrap_or_default());
                        let caption = self.content.take().unwrap_or_default();
                        let svg_url = typst_url.with_extension("svg");
                        self.exit();

                        let html = html_figure(&environment::full_url(&svg_url), true, caption);
                        return Some(Event::Html(html.into()));
                    }
                    State::ImageCode => {
                        let typst_url =
                            typst_path(self.current_slug, &self.url.take().unwrap_or_default());
                        let caption = self.content.take().unwrap_or_default();
                        let svg_url = typst_url.with_extension("svg");
                        self.exit();

                        let root_dir = environment::trees_dir();
                        let full_path = root_dir.join(typst_url);
                        let code = fs::read_to_string(format!("{}.code", full_path))
                            .or_else(|_| fs::read_to_string(&full_path))
                            .unwrap_or_else(|err| {
                                color_print::ceprintln!(
                                    "<y>Warning: failed to read typst source `{}`: {}</>",
                                    full_path,
                                    err
                                );
                                String::new()
                            });

                        let html =
                            html_figure_code(&environment::full_url(&svg_url), caption, code);
                        return Some(Event::Html(html.into()));
                    }
                    State::Shared => {
                        let Some(typst_url) = self.url.take() else {
                            color_print::ceprintln!(
                                "<y>Warning: missing shared typst url at `{}`.</>",
                                self.current_slug
                            );
                            self.state = State::None;
                            continue;
                        };
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

fn typst_path(current_slug: Slug, url: &str) -> Utf8PathBuf {
    let resolved = resolve_section_url(url, current_slug);
    let relocated = relocate_trees_path(&resolved);
    Utf8PathBuf::from(relocated.trim_start_matches('/'))
}

/// Reverses smart punctuation to plain ASCII characters.
fn smart_punctuation_reverse(s: &str) -> String {
    s.replace("“", "\"")
        .replace("”", "\"")
        .replace("‘", "'")
        .replace("’", "'")
        .replace("–", "--")
        .replace("—", "---")
}

#[cfg(test)]
mod tests {
    use super::typst_path;
    use crate::slug::Slug;
    use camino::Utf8PathBuf;

    #[test]
    fn test_typst_path_resolves_relative_paths() {
        crate::environment::mock_environment().unwrap();
        let path = typst_path(Slug::new("guide/chapter/index"), "../fig.typ");
        assert_eq!(path, Utf8PathBuf::from("guide/fig.typ"));
    }

    #[test]
    fn test_typst_path_relocates_trees_absolute_paths() {
        crate::environment::mock_environment().unwrap();
        let path = typst_path(Slug::new("guide/index"), "/trees/ref/plot.typ");
        assert_eq!(path, Utf8PathBuf::from("ref/plot.typ"));
    }

    #[test]
    fn test_typst_path_normalizes_dot_segments() {
        crate::environment::mock_environment().unwrap();
        let path = typst_path(Slug::new("a/b/index"), "./x/../y.typ");
        assert_eq!(path, Utf8PathBuf::from("a/b/y.typ"));
    }
}
