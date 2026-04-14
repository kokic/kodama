// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use super::{
    content::EventExtended,
    path_resolution::{relocate_trees_path_with_trees_root, resolve_section_url},
    processer::url_action,
};
use std::{
    fs, mem,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    compiler::section::{EmbedContent, HTMLContent, LocalLink, SectionOption},
    environment::{assets_dir_without_root, root_dir, trees_dir_without_root},
    html_flake::{html_code_block, html_link},
    path_utils,
    process::typst_image::is_inline_typst,
    recorder::State,
    slug::Slug,
};
use camino::Utf8PathBuf;
use pulldown_cmark::{html, Event, Tag, TagEnd};

static INCLUDE_READ_ERROR_FLAG: AtomicBool = AtomicBool::new(false);

pub fn reset_include_error_flag() {
    INCLUDE_READ_ERROR_FLAG.store(false, Ordering::Relaxed);
}

pub fn include_error_detected() -> bool {
    INCLUDE_READ_ERROR_FLAG.load(Ordering::Relaxed)
}

fn record_include_error() {
    INCLUDE_READ_ERROR_FLAG.store(true, Ordering::Relaxed);
}

pub struct Embed<'e, E> {
    events: E,
    current_slug: Slug,
    assets_dir_name: String,
    trees_dir_name: String,
    state: State,
    url: Option<String>,
    content: Vec<Event<'e>>,
}

impl<'e, E> Embed<'e, E> {
    pub fn process(events: E, current_slug: Slug) -> Self {
        Self::process_with_roots(
            events,
            current_slug,
            assets_dir_without_root(),
            trees_dir_without_root(),
        )
    }

    fn process_with_roots(
        events: E,
        current_slug: Slug,
        assets_dir_name: String,
        trees_dir_name: String,
    ) -> Self {
        Self {
            events,
            current_slug,
            assets_dir_name,
            trees_dir_name,
            state: State::None,
            url: None,
            content: Vec::new(),
        }
    }

    fn exit(&mut self) -> (String, Vec<Event<'e>>) {
        self.state = State::None;
        (
            self.url.take().unwrap_or_default(),
            mem::take(&mut self.content),
        )
    }
}

impl<'e, E: Iterator<Item = Event<'e>>> Iterator for Embed<'e, E> {
    type Item = EventExtended<'e>;

    fn next(&mut self) -> Option<Self::Item> {
        for e in self.events.by_ref() {
            match e {
                Event::Start(Tag::Link { ref dest_url, .. }) => {
                    let (url, action) = url_action(dest_url);
                    if !is_safe_link_target(&url) {
                        self.state = State::UnsafeLink;
                    } else if action == State::Embed.strify() {
                        self.state = State::Embed;
                        self.url = Some(resolve_embed_url_with_trees_root(
                            &url,
                            self.current_slug,
                            &self.trees_dir_name,
                        ));
                    // [0]
                    } else if action == State::Include.strify() {
                        self.state = State::Include;
                        self.url = Some(resolve_include_url(&url, self.current_slug));
                    } else if is_external_link(&url) {
                        self.state = State::ExternalLink;
                        self.url = Some(url);
                    } else if is_local_link_with_assets(dest_url, &self.assets_dir_name) {
                        self.state = State::LocalLink;
                        self.url = Some(resolve_local_link_url_with_trees_root(
                            &url,
                            self.current_slug,
                            &self.trees_dir_name,
                        ));
                    } else if is_assets_file_with_assets_root(&url, &self.assets_dir_name) {
                        self.state = State::AssetFile;
                        self.url = Some(url);
                    } else {
                        return Some(e.into());
                    }
                }
                Event::End(TagEnd::Link) => match self.state {
                    State::Embed => {
                        let (url, mut content) = self.exit();

                        let mut option = SectionOption::default();
                        let title = if let Some(e) = content.first_mut() {
                            // parse options, then strip /[-+.]/ from beginning of the title
                            if let Event::Text(t) = e {
                                let (opt, rest) = parse_embed_text(t);
                                option = opt;
                                *t = rest.into();
                            }
                            let mut title = String::new();
                            html::push_html(&mut title, content.into_iter());
                            Some(title)
                        } else {
                            None
                        };
                        let title = title.filter(|t| !t.is_empty());
                        return Some(EmbedContent { title, url, option }.into());
                    }
                    State::Include => {
                        let (url, content) = self.exit();
                        let language_tag = if content.is_empty() {
                            Some("plain".to_string())
                        } else {
                            let mut text = String::new();
                            html::push_html(&mut text, content.into_iter());
                            Some(text)
                        };

                        let include_path = root_dir().join(&url);
                        let content = fs::read_to_string(&include_path).unwrap_or_else(|err| {
                            record_include_error();
                            color_print::ceprintln!(
                                "<y>Warning: failed to include file `{}` resolved to `{}`: {}</>",
                                url,
                                include_path,
                                err
                            );
                            format!("failed to include file: {url}")
                        });
                        let escaped = htmlize::escape_text(content);
                        let html = html_code_block(&escaped, &language_tag.unwrap_or_default());
                        return Some(Event::Html(html.into()).into());
                    }
                    State::LocalLink => {
                        let (url, content) = self.exit();
                        let text = if content.is_empty() {
                            None
                        } else {
                            let mut text = String::new();
                            html::push_html(&mut text, content.into_iter());
                            Some(text)
                        };
                        return Some(LocalLink { url, text }.into());
                    }
                    State::ExternalLink => {
                        let (url, content) = self.exit();
                        let mut text = String::new();
                        html::push_html(&mut text, content.into_iter());
                        let formatted_title;
                        let title = if url == text {
                            &url
                        } else {
                            formatted_title = format!("{text} [{url}]");
                            &formatted_title
                        };
                        let html = html_link(&url, title, &text, State::ExternalLink.strify());
                        return Some(Event::Html(html.into()).into());
                    }
                    State::AssetFile => {
                        let (url, content) = self.exit();
                        let mut text = String::new();
                        html::push_html(&mut text, content.into_iter());
                        let html = html_link(&url, &text, &text, State::AssetFile.strify());
                        return Some(Event::Html(html.into()).into());
                    }
                    State::UnsafeLink => {
                        let (_, content) = self.exit();
                        let text = if content.is_empty() {
                            String::new()
                        } else {
                            let mut html = String::new();
                            html::push_html(&mut html, content.into_iter());
                            HTMLContent::Plain(html).remove_all_tags()
                        };
                        return Some(Event::Text(text.into()).into());
                    }
                    _ => return Some(e.into()),
                },
                Event::Text(_) if is_inline_allowed(&self.state) => self.content.push(e),
                Event::InlineHtml(_) if is_inline_allowed(&self.state) => self.content.push(e),
                Event::InlineMath(ref math) => {
                    let replaced = Event::Text(format!("${math}$").into());
                    if is_inline_allowed(&self.state) {
                        self.content.push(replaced);
                    } else {
                        return Some(replaced.into());
                    }
                }
                // TODO: move away from mangling math manually
                Event::DisplayMath(ref math) => {
                    return Some(Event::Text(format!("$${math}$$").into()).into())
                }
                Event::Code(_) if is_inline_allowed(&self.state) => {
                    self.content.push(e);
                }
                _ => return Some(e.into()),
            }
        }
        None
    }
}

fn resolve_embed_url_with_trees_root(
    raw_url: &str,
    current_slug: Slug,
    trees_dir_without_root: &str,
) -> String {
    let resolved = resolve_section_url(raw_url, current_slug);
    relocate_trees_path_with_trees_root(&resolved, trees_dir_without_root)
}

fn resolve_local_link_url_with_trees_root(
    raw_url: &str,
    current_slug: Slug,
    trees_dir_without_root: &str,
) -> String {
    let resolved = resolve_section_url(raw_url, current_slug);
    let resolved = strip_markdown_extension(&resolved);
    relocate_trees_path_with_trees_root(&resolved, trees_dir_without_root)
}

fn resolve_include_url(raw_url: &str, current_slug: Slug) -> String {
    let path = if raw_url.starts_with('/') {
        Utf8PathBuf::from(raw_url.trim_start_matches('/'))
    } else {
        path_utils::relative_to_current(current_slug.as_str(), raw_url)
    };
    path_utils::pretty_path(path.as_path())
}

fn strip_markdown_extension(url: &str) -> String {
    let mut path = Utf8PathBuf::from(url.trim_start_matches('/'));
    if path.extension() == Some("md") {
        path.set_extension("");
    }
    let pretty = path_utils::pretty_path(path.as_path());
    if pretty.is_empty() {
        "/".to_string()
    } else {
        format!("/{pretty}")
    }
}

fn parse_embed_text(embed_text: &str) -> (SectionOption, String) {
    let mut numbering = false;
    let mut details_open = true;
    let mut catalog = true;

    let mut index = 0;
    let chars = embed_text.chars();
    for curr in chars {
        match curr {
            '+' => numbering = true,
            '-' => details_open = false,
            '.' => catalog = false,
            _ => break,
        }
        index += 1;
    }

    let option = SectionOption::new(numbering, details_open, catalog);
    let inline_title = &embed_text[index..];
    (option, inline_title.to_owned())
}

/// Returns `true` if the current state allows inline elements such as `Text`, `Code`, and `InlineMath` to be included in the content buffer.
fn is_inline_allowed(state: &State) -> bool {
    *state == State::Embed
        || *state == State::Include
        || *state == State::LocalLink
        || *state == State::ExternalLink
        || *state == State::AssetFile
        || *state == State::UnsafeLink
}

fn is_safe_link_target(url: &str) -> bool {
    let trimmed = url.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return true;
    }

    let Some(scheme) = scheme_name(trimmed) else {
        return true;
    };
    !is_unsafe_scheme(&scheme) && is_allowed_scheme(&scheme)
}

fn is_external_link(url: &str) -> bool {
    let trimmed = url.trim();
    if trimmed.starts_with("www.") {
        return true;
    }
    scheme_name(trimmed).is_some_and(|scheme| is_allowed_scheme(&scheme))
}

fn scheme_name(url: &str) -> Option<String> {
    let scheme_end = url.find(':')?;
    if scheme_end == 0 {
        return None;
    }
    let first_delimiter = url.find(['/', '?', '#']).unwrap_or(url.len());
    if scheme_end > first_delimiter {
        return None;
    }
    let scheme = &url[..scheme_end];
    if scheme
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
    {
        return Some(scheme.to_ascii_lowercase());
    }
    None
}

fn is_allowed_scheme(scheme: &str) -> bool {
    matches!(scheme, "http" | "https" | "ftp" | "mailto")
}

fn is_unsafe_scheme(scheme: &str) -> bool {
    matches!(scheme, "javascript" | "vbscript" | "data" | "file")
}

/// Returns `true` if the URL represents a static asset file in the configured assets directory.
fn is_assets_file_with_assets_root(url: &str, assets_dir_without_root: &str) -> bool {
    let assets_root = normalize_path_prefix(assets_dir_without_root);
    if assets_root.is_empty() {
        return false;
    }

    let normalized_url = normalize_path_prefix(url);
    normalized_url == assets_root || normalized_url.starts_with(&format!("{assets_root}/"))
}

/// Returns `true` if the URL represents a local wiki link.
fn is_local_link_with_assets(url: &str, assets_dir_without_root: &str) -> bool {
    !url.ends_with("/")
        && !is_inline_typst(url)
        && !is_external_link(url)
        && !is_assets_file_with_assets_root(url, assets_dir_without_root)
        && !url.contains(":")
}

fn normalize_path_prefix(path: &str) -> String {
    let mut normalized = path.replace('\\', "/");
    while let Some(rest) = normalized.strip_prefix("./") {
        normalized = rest.to_string();
    }
    normalized.trim_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::{content::EventExtended, text_elaborator::TextElaborator};
    use pulldown_cmark::{Event, Parser};

    const ASSETS_DIR: &str = "assets";
    const TREES_DIR: &str = "trees";

    #[test]
    fn test_is_assets_file() {
        assert!(is_assets_file_with_assets_root(
            "assets/image.png",
            ASSETS_DIR
        ));
        assert!(is_assets_file_with_assets_root(
            "/assets/image.png",
            ASSETS_DIR
        ));
        assert!(is_assets_file_with_assets_root(
            "\\assets\\image.png",
            ASSETS_DIR
        ));

        assert!(!is_assets_file_with_assets_root("image.png", ASSETS_DIR));
        assert!(!is_assets_file_with_assets_root(
            "path/to/assets/image.png",
            ASSETS_DIR
        ));
        assert!(!is_assets_file_with_assets_root(
            "/path/to/image.png",
            ASSETS_DIR
        ));
    }

    #[test]
    fn test_is_local_link() {
        assert!(is_local_link_with_assets("./0AB7", ASSETS_DIR));
        assert!(is_local_link_with_assets("./0AB7.md", ASSETS_DIR));
        assert!(is_local_link_with_assets("/path/to/0AB7", ASSETS_DIR));

        assert!(!is_local_link_with_assets("http://example.com", ASSETS_DIR));
        assert!(!is_local_link_with_assets(
            "https://example.com",
            ASSETS_DIR
        ));
        assert!(!is_local_link_with_assets("www.example.com", ASSETS_DIR));
        assert!(!is_local_link_with_assets("external:page", ASSETS_DIR));
        assert!(!is_local_link_with_assets("inline", ASSETS_DIR));
        assert!(!is_local_link_with_assets("inline-0pt-0pt", ASSETS_DIR));
        assert!(!is_local_link_with_assets("assets/image.png", ASSETS_DIR));
        assert!(!is_local_link_with_assets("/assets/image.png", ASSETS_DIR));
        assert!(!is_local_link_with_assets("local-dir/", ASSETS_DIR));
    }

    #[test]
    fn test_resolve_include_url_supports_root_and_relative_paths() {
        assert_eq!(
            resolve_include_url("/import-meta.html", Slug::new("a/b")),
            "import-meta.html"
        );
        assert_eq!(
            resolve_include_url("./shared/snippet.txt", Slug::new("docs/chapter")),
            "docs/shared/snippet.txt"
        );
        assert_eq!(
            resolve_include_url("../snippet.txt", Slug::new("docs/chapter")),
            "snippet.txt"
        );
    }

    #[test]
    fn test_resolve_local_and_embed_urls_are_normalized_early() {
        assert_eq!(
            resolve_local_link_url_with_trees_root("./a.b.md", Slug::new("guide/index"), TREES_DIR),
            "/guide/a.b"
        );
        assert_eq!(
            resolve_embed_url_with_trees_root(
                "../ref.md",
                Slug::new("guide/chapter/page"),
                TREES_DIR
            ),
            "/guide/ref.md"
        );
        assert_eq!(
            resolve_local_link_url_with_trees_root(
                "/trees/root.md",
                Slug::new("guide/index"),
                TREES_DIR
            ),
            "/root"
        );
    }

    #[test]
    fn test_is_safe_link_target_filters_unsafe_schemes() {
        assert!(is_safe_link_target("https://example.com"));
        assert!(is_safe_link_target("./target"));
        assert!(is_safe_link_target("/target"));
        assert!(is_safe_link_target("mailto:dev@example.com"));

        assert!(!is_safe_link_target("javascript:alert(1)"));
        assert!(!is_safe_link_target("vbscript:msgbox(1)"));
        assert!(!is_safe_link_target("data:text/html,<svg/onload=alert(1)>"));
        assert!(!is_safe_link_target("file:///etc/passwd"));
    }

    #[test]
    fn test_unsafe_link_is_downgraded_to_plain_text_event() {
        let source = "[click](javascript:alert(1))";
        let events = Parser::new_ext(source, crate::compiler::parser::OPTIONS);
        let events = TextElaborator::process(events);
        let actual = Embed::process_with_roots(
            events,
            Slug::new("index"),
            ASSETS_DIR.to_string(),
            TREES_DIR.to_string(),
        )
        .collect::<Vec<_>>();

        assert!(actual
            .iter()
            .any(|event| matches!(event, EventExtended::CMark(Event::Text(text)) if text.as_ref() == "click")));
        assert!(!actual.iter().any(|event| {
            matches!(
                event,
                EventExtended::CMark(Event::Html(_)) | EventExtended::CMark(Event::InlineHtml(_))
            )
        }));
    }

    #[test]
    fn test_local_link_keeps_text_elaborator_inline_html_in_link_text() {
        let source = "[中文](./target)";
        let events = Parser::new_ext(source, crate::compiler::parser::OPTIONS);
        let events = TextElaborator::process(events);
        let actual = Embed::process_with_roots(
            events,
            Slug::new("index"),
            ASSETS_DIR.to_string(),
            TREES_DIR.to_string(),
        )
        .collect::<Vec<_>>();

        assert_eq!(
            actual
                .iter()
                .filter(|event| matches!(event, EventExtended::Local(_)))
                .count(),
            1
        );
        assert!(!actual
            .iter()
            .any(|event| matches!(event, EventExtended::CMark(Event::InlineHtml(_)))));

        let local_link = actual
            .iter()
            .find_map(|event| match event {
                EventExtended::Local(local_link) => Some(local_link),
                _ => None,
            })
            .expect("expected a local link event");
        assert_eq!(local_link.url, "/target");
        assert_eq!(
            local_link.text.as_deref(),
            Some(r#"<span lang="zh">中文</span>"#)
        );
    }

    #[test]
    fn test_asset_link_title_strips_text_elaborator_inline_html() {
        let source = "[中文](/assets/image.png)";
        let events = Parser::new_ext(source, crate::compiler::parser::OPTIONS);
        let events = TextElaborator::process(events);
        let actual = Embed::process_with_roots(
            events,
            Slug::new("index"),
            ASSETS_DIR.to_string(),
            TREES_DIR.to_string(),
        )
        .collect::<Vec<_>>();

        let html = actual
            .iter()
            .find_map(|event| match event {
                EventExtended::CMark(Event::Html(html)) => Some(html.as_ref()),
                _ => None,
            })
            .expect("expected an html link event");

        assert!(html.contains(r#"class="link asset""#));
        assert!(html.contains(r#"href="/assets/image.png""#));
        assert!(html.contains(r#"title="中文""#));
        assert!(!html.contains(r#"title="<span"#));
        assert!(!html.contains("&lt;span"));
        assert!(html.contains(r#"><span lang="zh">中文</span></a>"#));
    }
}
