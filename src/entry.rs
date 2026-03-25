// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use crate::{
    compiler::{section::HTMLContent, taxon::Taxon},
    config::build::FooterMode,
    environment, html_flake,
    ordered_map::OrderedMap,
    slug::Slug,
};
use eyre::eyre;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HTMLMetaData(pub OrderedMap<String, HTMLContent>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryMetaData(pub OrderedMap<String, String>);

pub const KEY_TITLE: &str = "title";

/// Auto-detected
pub const KEY_SLUG: &str = "slug";

/// Auto-detected
pub const KEY_EXT: &str = "ext";

pub const KEY_TAXON: &str = "taxon";
pub const KEY_DATA_TAXON: &str = "data-taxon";

/// Control the "Previous Level" information in the current page navigation.
pub const KEY_PARENT: &str = "parent";

/// Control the page title text of the current page.
pub const KEY_PAGE_TITLE: &str = "page-title";
pub const KEY_SOURCE_SLUG: &str = "source-slug";
pub const KEY_SOURCE_POS: &str = "source-pos";
pub const KEY_INTERNAL_ANON_SUBTREE: &str = "internal-anon-subtree";

/// `backlinks: bool`:
/// Controls whether the current page displays backlinks.
pub const KEY_BACKLINKS: &str = "backlinks";

/// `transparent-backlinks: bool`:
/// Controls whether backlinks of current section is always displayed,
/// even when embedded (except in footer).
/// Default is `false`.
pub const KEY_TRANSPARENT_BACKLINKS: &str = "transparent-backlinks";

/// `references: bool`:
/// Controls whether the current page displays references.
pub const KEY_REFERENCES: &str = "references";

/// `collect: bool`:
/// Controls whether the current page is a collection page.
/// A collection page displays metadata of child entries.
pub const KEY_COLLECT: &str = "collect";

/// `asref: bool`:
/// Controls whether the current page process as reference.
/// Default is `false`.
pub const KEY_ASREF: &str = "asref";

/// `asback: bool`:
/// Controls whether the current page process as backlink.
/// Default is `true`.
pub const KEY_ASBACK: &str = "asback";

/// `footer-mode: embed | link`
pub const KEY_FOOTER_MODE: &str = "footer-mode";

/// `footer-sort-by: <metadata-key>`
pub const KEY_FOOTER_SORT_BY: &str = "footer-sort-by";

const FANCY_METADATA: [&str; 2] = [KEY_TITLE, KEY_TAXON];

const PLAIN_METADATA: [&str; 16] = [
    KEY_SLUG,
    KEY_EXT,
    KEY_DATA_TAXON,
    KEY_PARENT,
    KEY_PAGE_TITLE,
    KEY_SOURCE_SLUG,
    KEY_SOURCE_POS,
    KEY_INTERNAL_ANON_SUBTREE,
    KEY_BACKLINKS,
    KEY_TRANSPARENT_BACKLINKS,
    KEY_REFERENCES,
    KEY_COLLECT,
    KEY_ASREF,
    KEY_ASBACK,
    KEY_FOOTER_MODE,
    KEY_FOOTER_SORT_BY,
];

pub fn is_plain_metadata(s: &str) -> bool {
    PLAIN_METADATA.contains(&s)
}

pub fn is_fancy_metadata(s: &str) -> bool {
    FANCY_METADATA.contains(&s)
}

pub fn is_custom_metadata(s: &str) -> bool {
    !is_plain_metadata(s) && !is_fancy_metadata(s)
}

pub trait MetaData<V>
where
    V: Clone,
{
    fn get(&self, key: &str) -> Option<&V>;
    fn get_str(&self, key: &str) -> Option<&String>;
    fn keys(&self) -> impl Iterator<Item = &String>;

    /// Return all custom metadata keys.
    fn etc_keys(&self) -> Vec<String> {
        self.keys()
            .filter(|s| is_custom_metadata(s))
            .map(|s| s.to_string())
            .collect()
    }

    /// Return all custom metadata values.
    fn etc(&self) -> Vec<V> {
        self.etc_keys()
            .into_iter()
            .filter_map(|s| self.get(&s).cloned())
            .collect()
    }

    fn get_bool(&self, key: &str) -> eyre::Result<Option<bool>> {
        let Some(value) = self.get_str(key) else {
            return Ok(None);
        };
        match value.as_str() {
            "true" => Ok(Some(true)),
            "false" => Ok(Some(false)),
            _ => {
                let slug = self
                    .get_str(KEY_SLUG)
                    .map(String::as_str)
                    .unwrap_or("<unknown>");
                Err(eyre!(
                    "invalid bool metadata in `{}`: `{}` = `{}` (expected `true` or `false`)",
                    slug,
                    key,
                    value
                ))
            }
        }
    }

    fn id(&self) -> eyre::Result<String> {
        let slug = self
            .get_str(KEY_SLUG)
            .ok_or_else(|| eyre!("missing required metadata `slug` while rendering section id"))?;
        Ok(crate::slug::to_hash_id(slug))
    }

    /// Return taxon text
    fn taxon(&self) -> Option<&V> {
        self.get(KEY_TAXON)
    }

    fn data_taxon(&self) -> Option<&String> {
        self.get_str(KEY_DATA_TAXON)
    }

    fn parent(&self) -> Option<Slug> {
        self.get_str(KEY_PARENT).map(Slug::new)
    }

    fn title(&self) -> Option<&V> {
        self.get(KEY_TITLE)
    }

    fn page_title(&self) -> Option<&String> {
        self.get_str(KEY_PAGE_TITLE)
    }

    fn slug(&self) -> Option<Slug> {
        self.get_str(KEY_SLUG).map(Slug::new)
    }

    fn ext(&self) -> Option<&String> {
        self.get_str(KEY_EXT)
    }

    fn backlinks_enabled(&self) -> eyre::Result<bool> {
        self.get_bool(KEY_BACKLINKS).map(|v| v.unwrap_or(true))
    }

    fn is_backlinks_transparent(&self) -> eyre::Result<bool> {
        self.get_bool(KEY_TRANSPARENT_BACKLINKS)
            .map(|v| v.unwrap_or(false))
    }

    fn references_enabled(&self) -> eyre::Result<bool> {
        self.get_bool(KEY_REFERENCES).map(|v| v.unwrap_or(true))
    }

    fn is_collect(&self) -> eyre::Result<bool> {
        self.get_bool(KEY_COLLECT).map(|v| v.unwrap_or(false))
    }

    fn is_asref(&self) -> eyre::Result<Option<bool>> {
        self.get_bool(KEY_ASREF)
    }

    fn is_asback(&self) -> eyre::Result<Option<bool>> {
        self.get_bool(KEY_ASBACK)
    }
}

impl MetaData<HTMLContent> for HTMLMetaData {
    fn get(&self, key: &str) -> Option<&HTMLContent> {
        self.0.get(key)
    }

    fn get_str(&self, key: &str) -> Option<&String> {
        self.0.get(key).and_then(HTMLContent::as_string)
    }

    fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.keys()
    }
}

impl MetaData<String> for EntryMetaData {
    fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    fn get_str(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.keys()
    }
}

impl HTMLMetaData {
    pub fn compute_textual_attrs(&mut self) {
        if self.page_title().is_none() {
            if let Some(title) = self.title() {
                self.0.insert(
                    KEY_PAGE_TITLE.to_string(),
                    HTMLContent::Plain(title.remove_all_tags()),
                );
            }
        }

        if self.data_taxon().is_none() {
            if let Some(taxon) = self.taxon() {
                self.0.insert(
                    KEY_DATA_TAXON.to_string(),
                    HTMLContent::Plain(Taxon::to_data_taxon(&taxon.remove_all_tags()).to_string()),
                );
            }
        }
    }
}

impl EntryMetaData {
    pub fn to_header(
        &self,
        adhoc_title: Option<&str>,
        adhoc_taxon: Option<&str>,
    ) -> eyre::Result<String> {
        let entry_taxon = self.taxon().map_or("", |s| s);
        let taxon = adhoc_taxon.unwrap_or(entry_taxon);
        let entry_title = self.0.get("title").map(|s| s.as_str()).unwrap_or("");
        let title = adhoc_title.unwrap_or(entry_title);
        let slug = self
            .slug()
            .ok_or_else(|| eyre!("missing required metadata `slug` while rendering header"))?;
        let ext = self.ext().map(String::as_str).ok_or_else(|| {
            eyre!(
                "missing required metadata `ext` while rendering header for `{}`",
                slug
            )
        })?;
        let show_slug = !self.get_bool(KEY_INTERNAL_ANON_SUBTREE)?.unwrap_or(false);
        let etc = self.etc();

        Ok(html_flake::html_header(html_flake::HtmlHeaderArgs {
            title,
            taxon,
            slug: &slug,
            ext,
            show_slug,
            source_slug: self.get_str(KEY_SOURCE_SLUG).map(String::as_str),
            source_pos: self.get_str(KEY_SOURCE_POS).map(String::as_str),
            etc: &etc,
        }))
    }

    /// hidden suffix `/index` in slug text.
    pub fn to_slug_text(slug: &str) -> String {
        let mut slug_text = match slug.ends_with("/index") {
            true => &slug[..slug.len() - "/index".len()],
            false => slug,
        };
        if environment::is_short_slug() {
            let pos = slug_text.rfind("/").map_or(0, |n| n + 1);
            slug_text = &slug_text[pos..];
        }
        slug_text.to_string()
    }

    pub fn update(&mut self, key: String, value: String) {
        let _ = self.0.insert(key, value);
    }

    pub fn footer_mode(&self) -> eyre::Result<Option<FooterMode>> {
        let Some(value) = self.get_str(KEY_FOOTER_MODE) else {
            return Ok(None);
        };
        value.parse().map(Some).map_err(|_| {
            let slug = self
                .get_str(KEY_SLUG)
                .map(String::as_str)
                .unwrap_or("<unknown>");
            eyre!(
                "invalid metadata in `{}`: `footer-mode = {}` (expected `embed` or `link`)",
                slug,
                value
            )
        })
    }

    pub fn footer_sort_by(&self) -> Option<String> {
        self.get_str(KEY_FOOTER_SORT_BY)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }
}
