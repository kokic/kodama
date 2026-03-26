// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use camino::{Utf8Path, Utf8PathBuf};

use crate::config::{build::FooterMode, kodama, toc};

use super::{with_config, with_environment, BuildMode, CACHE_DIR_NAME};

pub fn is_short_slug() -> bool {
    with_config(|cfg| cfg.build.short_slug)
}

pub fn typst_root_dir() -> Utf8PathBuf {
    with_config(|cfg| cfg.build.typst_root.clone().into())
}

pub fn trees_dir_without_root() -> String {
    with_config(|cfg| cfg.kodama.trees.clone())
}

pub fn assets_dir_without_root() -> String {
    with_config(|cfg| cfg.kodama.assets.clone())
}

pub fn trees_dir() -> Utf8PathBuf {
    super::root_dir().join(trees_dir_without_root())
}

pub fn theme_paths() -> Vec<Utf8PathBuf> {
    let root = super::root_dir();
    with_config(|cfg| {
        cfg.kodama
            .themes
            .iter()
            .map(|theme| root.join(theme))
            .collect()
    })
}

pub fn output_dir() -> Utf8PathBuf {
    with_environment(|env| {
        let output = match env.build_mode {
            BuildMode::Publish | BuildMode::Check => env.config.build.output.clone(),
            BuildMode::Serve => env.config.serve.output.clone(),
        };
        env.root.join(output)
    })
}

pub fn indexes_path(output_dir: &Utf8Path) -> Utf8PathBuf {
    output_dir.join("kodama.json")
}

pub fn graph_path(output_dir: &Utf8Path) -> Utf8PathBuf {
    output_dir.join("kodama.graph.json")
}

pub fn feed_path(output_dir: &Utf8Path) -> Utf8PathBuf {
    output_dir.join("feed.xml")
}

pub fn reload_marker_path(output_dir: &Utf8Path) -> Utf8PathBuf {
    output_dir.join("kodama.reload")
}

pub fn base_url_raw() -> String {
    with_config(|cfg| cfg.kodama.base_url.clone())
}

pub fn base_url() -> String {
    with_environment(|env| match env.build_mode {
        BuildMode::Publish | BuildMode::Check => env.config.kodama.base_url.clone(),
        BuildMode::Serve => kodama::DEFAULT_BASE_URL.to_string(),
    })
}

pub fn is_toc_left() -> bool {
    match with_config(|cfg| cfg.toc.placement) {
        toc::TocPlacement::Left => true,
        toc::TocPlacement::Right => false,
    }
}

pub fn is_toc_sticky() -> bool {
    with_config(|cfg| cfg.toc.sticky)
}

pub fn is_toc_mobile_sticky() -> bool {
    with_config(|cfg| cfg.toc.mobile_sticky)
}

pub fn toc_max_width() -> String {
    with_config(|cfg| cfg.toc.max_width.clone())
}

pub fn get_edit_text() -> String {
    with_config(|cfg| cfg.text.edit.clone())
}

pub fn get_toc_text() -> String {
    with_config(|cfg| cfg.text.toc.clone())
}

pub fn get_footer_references_text() -> String {
    with_config(|cfg| cfg.text.references.clone())
}

pub fn get_footer_backlinks_text() -> String {
    with_config(|cfg| cfg.text.backlinks.clone())
}

pub fn footer_mode() -> FooterMode {
    with_config(|cfg| cfg.build.footer_mode)
}

pub fn footer_sort_by() -> String {
    with_config(|cfg| cfg.build.footer_sort_by.clone())
}

pub fn publish_rss() -> bool {
    with_config(|cfg| cfg.publish.rss)
}

pub fn inline_css() -> bool {
    with_config(|cfg| cfg.build.inline_css)
}

pub fn inline_script() -> bool {
    with_config(|cfg| cfg.build.inline_script)
}

pub fn asref() -> bool {
    with_config(|cfg| cfg.build.asref)
}

pub fn deploy_edit_url() -> Option<String> {
    with_config(|cfg| cfg.build.edit.clone())
}

pub fn editor_url() -> Option<String> {
    with_config(|cfg| cfg.serve.edit.clone())
}

pub fn serve_command() -> Vec<String> {
    with_config(|cfg| cfg.serve.command.clone())
}

pub fn get_cache_dir() -> Utf8PathBuf {
    super::root_dir().join(CACHE_DIR_NAME)
}

pub fn assets_dir() -> Utf8PathBuf {
    super::root_dir().join(assets_dir_without_root())
}
