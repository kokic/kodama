// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use eyre::eyre;
use std::{collections::HashSet, ops::Not};

use crate::{
    compiler::counter::Counter,
    config::build::FooterMode,
    entry::{MetaData, KEY_INTERNAL_ANON_SUBTREE},
    environment::{self, verify_update_hash},
    html_flake::{self, html_footer_section},
    slug::Slug,
};

use super::{
    callback::CallbackValue,
    section::{Section, SectionContent},
    state::CompileState,
    taxon::Taxon,
};

pub struct Writer {}

impl Writer {
    pub fn write(section: &Section, state: &CompileState) -> eyre::Result<()> {
        let (html, page_title) = Writer::html_doc(section, state)?;
        let relative_path = format!("{}.html", section.slug()?);
        let filepath = crate::environment::output_path(&relative_path);

        match verify_update_hash(&relative_path, &html) {
            Ok(true) => match std::fs::write(&filepath, html) {
                Ok(()) => {
                    if *crate::cli::build::verbose() {
                        color_print::ceprintln!("<g>[build]</> {:?} {}", page_title, filepath);
                    }
                }
                Err(err) => color_print::ceprintln!("<r>{:?}</>", err),
            },
            Ok(false) => {}
            Err(err) => {
                color_print::ceprintln!(
                    "<y>Warning: failed to verify hash for `{}`: {}</>",
                    relative_path,
                    err
                );
            }
        }

        Ok(())
    }

    pub fn write_needed_slugs<I>(all_slugs: I, state: &CompileState) -> eyre::Result<()>
    where
        I: IntoIterator<Item = Slug>,
    {
        for slug in all_slugs {
            let section = state
                .compiled()
                .get(&slug)
                .ok_or_else(|| eyre!("slug `{}` not in compiled entries", slug))?;
            Writer::write(section, state)?;
        }
        Ok(())
    }

    pub fn html_doc(section: &Section, state: &CompileState) -> eyre::Result<(String, String)> {
        let mut counter = Counter::init();

        let (article_inner, items) =
            Writer::section_to_html(section, &mut counter, true, false, state)?;
        let catalog_html = if items.is_empty().not() {
            html_flake::html_catalog_block(&items)
        } else {
            Default::default()
        };

        let slug = section.slug()?;
        let html_header = Writer::header(state, slug);

        let callback = state.callback().0.get(&slug);
        let footer_sort_by = section
            .metadata
            .footer_sort_by()
            .unwrap_or_else(environment::footer_sort_by);
        let footer_html = Writer::footer(
            section.metadata.footer_mode()?,
            section.metadata.references_enabled()?,
            &footer_sort_by,
            state,
            &section.references,
            callback,
        )?;
        let page_title = section.metadata.page_title().map_or("", |s| s.as_str());

        let html = crate::html_flake::html_doc(
            page_title,
            &html_header,
            &article_inner,
            &footer_html,
            &catalog_html,
        );

        Ok((html, page_title.to_string()))
    }

    fn header(state: &CompileState, slug: Slug) -> String {
        // We must avoid section `index` defaulting to itself as its parent section.
        if slug.as_str() == "index" {
            return String::default();
        }

        let parent = state
            .callback()
            .0
            .get(&slug)
            .map_or(Slug::new("index"), |callback| callback.parent);
        let Some(section) = state.compiled().get(&parent) else {
            color_print::ceprintln!(
                "<y>Warning: missing parent section `{}` for `{}`; header nav is skipped.</>",
                parent,
                slug
            );
            return String::default();
        };

        let href = environment::full_html_url(parent);
        let title = section.metadata.title().map_or("", |s| s);
        let page_title = section.metadata.page_title().map_or("", |s| s);
        html_flake::html_header_nav(title, page_title, &href)
    }

    fn footer(
        footer_mode: Option<FooterMode>,
        enable_references: bool,
        footer_sort_by: &str,
        state: &CompileState,
        references: &HashSet<Slug>,
        callback: Option<&CallbackValue>,
    ) -> eyre::Result<String> {
        let mut references: Vec<Slug> = references.iter().copied().collect();
        Writer::sort_footer_slugs(&mut references, state, footer_sort_by);

        let references_text = environment::get_footer_references_text();
        let references_html = if enable_references {
            let mut content = String::new();
            for slug in &references {
                let Some(section) = state.compiled().get(slug) else {
                    color_print::ceprintln!(
                        "<y>Warning: missing referenced section `{}`; skipping footer reference.</>",
                        slug
                    );
                    continue;
                };
                content.push_str(&Writer::footer_section_to_html(footer_mode, section)?);
            }

            if content.is_empty() {
                String::default()
            } else {
                html_footer_section("references", &references_text, &content)
            }
        } else {
            String::default()
        };

        let backlinks_text = environment::get_footer_backlinks_text();
        let backlinks_html = if let Some(s) = callback {
            let mut backlinks: Vec<Slug> = s.backlinks.iter().copied().collect();
            Writer::sort_footer_slugs(&mut backlinks, state, footer_sort_by);
            let mut content = String::new();
            for slug in backlinks {
                let Some(section) = state.compiled().get(&slug) else {
                    color_print::ceprintln!(
                        "<y>Warning: missing backlink section `{}`; skipping footer backlink.</>",
                        slug
                    );
                    continue;
                };
                content.push_str(&Writer::footer_section_to_html(footer_mode, section)?);
            }

            if content.is_empty() {
                String::default()
            } else {
                html_footer_section("backlinks", &backlinks_text, &content)
            }
        } else {
            String::default()
        };
        Ok(html_flake::html_footer(&references_html, &backlinks_html))
    }

    fn sort_footer_slugs(slugs: &mut Vec<Slug>, state: &CompileState, footer_sort_by: &str) {
        slugs.sort_by(|left, right| {
            let left_section = state.compiled().get(left);
            let right_section = state.compiled().get(right);
            let left_value = left_section.map_or("", |section| {
                Writer::footer_sort_value(footer_sort_by, section, left)
            });
            let right_value = right_section.map_or("", |section| {
                Writer::footer_sort_value(footer_sort_by, section, right)
            });

            left_value.cmp(right_value).then_with(|| left.cmp(right))
        });
    }

    fn footer_sort_value<'a>(
        footer_sort_by: &str,
        section: &'a Section,
        slug: &'a Slug,
    ) -> &'a str {
        match footer_sort_by.trim() {
            "slug" => slug.as_str(),
            "date" => section.metadata.get_str("date").map_or("", String::as_str),
            "taxon" => section.metadata.data_taxon().map_or("", String::as_str),
            "title" => section.metadata.title().map_or("", String::as_str),
            key => section.metadata.get_str(key).map_or("", String::as_str),
        }
    }

    fn catalog_item(section: &Section, taxon: &str, child_html: &str) -> eyre::Result<String> {
        let slug = section.slug()?;
        let title = section.metadata.title().map_or("", |s| s);
        let page_title = section.metadata.page_title().map_or("", |s| s);
        let use_hash_href = Writer::is_internal_anonymous_subtree(section)?;
        Ok(html_flake::catalog_item(
            slug,
            title,
            page_title,
            section.option.details_open,
            taxon,
            child_html,
            use_hash_href,
        ))
    }

    fn footer_content_to_html(
        page_option: Option<FooterMode>,
        content: &SectionContent,
    ) -> eyre::Result<String> {
        match content {
            SectionContent::Plain(s) => Ok(s.to_string()),
            SectionContent::Embed(section) => Writer::footer_section_to_html(page_option, section),
        }
    }

    fn footer_section_to_html(
        page_option: Option<FooterMode>,
        section: &Section,
    ) -> eyre::Result<String> {
        let footer_mode = page_option.unwrap_or(environment::footer_mode());

        match footer_mode {
            FooterMode::Link => {
                let summary = section.metadata.to_header(None, None)?;
                let data_taxon = section.metadata.data_taxon().map_or("", |s| s);
                Ok(format!(
                    r#"<section class="block" data-taxon="{data_taxon}" style="margin-bottom: 0.4em;">{summary}</section>"#
                ))
            }
            FooterMode::Embed => {
                let mut contents = String::new();
                for content in &section.children {
                    contents.push_str(&Writer::footer_content_to_html(page_option, content)?);
                }
                html_flake::html_article_inner(
                    &section.metadata,
                    &contents,
                    false,
                    false,
                    None,
                    None,
                )
            }
        }
    }

    pub fn section_to_html(
        section: &Section,
        counter: &mut Counter,
        toplevel: bool,
        hide_metadata: bool,
        state: &CompileState,
    ) -> eyre::Result<(String, String)> {
        let adhoc_taxon = Writer::taxon(section, counter);
        let (mut contents, mut items) = (String::new(), String::new());

        if !section.children.is_empty() {
            let mut subcounter = match section.option.numbering {
                true => counter.left_shift(),
                false => counter.clone(),
            };
            let is_collection = section.metadata.is_collect()?;

            for child in &section.children {
                let (content_html, item_html) =
                    Writer::content_to_html(child, &mut subcounter, !is_collection, state)?;
                contents.push_str(&content_html);
                items.push_str(&item_html);
            }
        };

        if !toplevel && section.metadata.is_backlinks_transparent()? {
            let slug = section.slug()?;
            let footer_sort_by = section
                .metadata
                .footer_sort_by()
                .unwrap_or_else(environment::footer_sort_by);
            let backlinks_html = Writer::footer(
                section.metadata.footer_mode()?,
                false,
                &footer_sort_by,
                state,
                &section.references,
                state.callback().0.get(&slug),
            )?;
            contents += &backlinks_html;
        }

        let child_html = if !items.is_empty() {
            format!(r#"<ul class="block">{}</ul>"#, &items)
        } else {
            String::default()
        };

        let catalog_item = if toplevel {
            child_html
        } else {
            section
                .option
                .catalog
                .then(|| Writer::catalog_item(section, &adhoc_taxon, &child_html))
                .transpose()?
                .unwrap_or(String::new())
        };

        let article_inner = html_flake::html_article_inner(
            &section.metadata,
            &contents,
            hide_metadata,
            section.option.details_open,
            None,
            Some(adhoc_taxon.as_str()),
        )?;

        Ok((article_inner, catalog_item))
    }

    fn content_to_html(
        content: &SectionContent,
        counter: &mut Counter,
        hide_metadata: bool,
        state: &CompileState,
    ) -> eyre::Result<(String, String)> {
        match content {
            SectionContent::Plain(s) => Ok((s.to_string(), String::new())),
            SectionContent::Embed(section) => {
                Writer::section_to_html(section, counter, false, hide_metadata, state)
            }
        }
    }

    fn taxon(section: &Section, counter: &mut Counter) -> String {
        if section.option.numbering {
            counter.step_mut();
            let numbering = Some(counter.display());
            let text = section.metadata.taxon().map_or("", |s| s);
            let taxon = Taxon::new(numbering, text.to_string());
            return taxon.display();
        }
        section.metadata.taxon().map_or("", |s| s).to_string()
    }

    fn is_internal_anonymous_subtree(section: &Section) -> eyre::Result<bool> {
        Ok(section
            .metadata
            .get_bool(KEY_INTERNAL_ANON_SUBTREE)?
            .unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        compiler::{
            section::{EmbedContent, HTMLContent, LazyContent, SectionOption, UnresolvedSection},
            state::compile_all,
        },
        entry::{
            HTMLMetaData, KEY_EXT, KEY_INTERNAL_ANON_SUBTREE, KEY_PAGE_TITLE, KEY_REFERENCES,
            KEY_SLUG, KEY_TITLE,
        },
        ordered_map::OrderedMap,
    };

    use super::*;

    fn with_test_env(f: impl FnOnce()) {
        let root = crate::test_io::case_dir("writer");
        std::fs::create_dir_all(root.as_std_path()).unwrap();
        crate::environment::with_test_environment(
            root.clone(),
            crate::environment::BuildMode::Build,
            f,
        );
        let _ = std::fs::remove_dir_all(root.as_std_path());
    }

    fn shallow_section(slug: &str, title: &str) -> UnresolvedSection {
        shallow_section_with_content(slug, title, HTMLContent::Plain("<p>hello</p>".to_string()))
    }

    fn shallow_section_with_content(
        slug: &str,
        title: &str,
        content: HTMLContent,
    ) -> UnresolvedSection {
        let mut metadata = OrderedMap::new();
        metadata.insert(KEY_SLUG.to_string(), HTMLContent::Plain(slug.to_string()));
        metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain("md".to_string()));
        metadata.insert(KEY_TITLE.to_string(), HTMLContent::Plain(title.to_string()));
        metadata.insert(
            KEY_PAGE_TITLE.to_string(),
            HTMLContent::Plain(title.to_string()),
        );

        UnresolvedSection {
            metadata: HTMLMetaData(metadata),
            content,
        }
    }

    #[test]
    fn test_html_doc_skips_header_when_parent_section_is_missing() {
        with_test_env(|| {
            let mut shallows = HashMap::new();
            shallows.insert(Slug::new("a"), shallow_section("a", "A"));

            let state = compile_all(&shallows).unwrap();
            let section = state.compiled().get(&Slug::new("a")).unwrap();
            let (html, _title) = Writer::html_doc(section, &state).unwrap();

            assert!(!html.contains(r#"class="header""#));
        });
    }

    #[test]
    fn test_html_doc_returns_error_for_invalid_bool_metadata() {
        with_test_env(|| {
            let mut shallows = HashMap::new();
            let mut section = shallow_section("a", "A");
            section.metadata.0.insert(
                KEY_REFERENCES.to_string(),
                HTMLContent::Plain("maybe".to_string()),
            );
            shallows.insert(Slug::new("a"), section);

            let state = compile_all(&shallows).unwrap();
            let section = state.compiled().get(&Slug::new("a")).unwrap();
            let err = Writer::html_doc(section, &state).unwrap_err();

            assert!(err.to_string().contains("invalid bool metadata"));
        });
    }

    #[test]
    fn test_html_doc_toc_uses_hash_link_for_internal_anonymous_subtree() {
        with_test_env(|| {
            let mut shallows = HashMap::new();
            shallows.insert(
                Slug::new("index"),
                shallow_section_with_content(
                    "index",
                    "Root",
                    HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                        url: "/anon".to_string(),
                        title: None,
                        option: SectionOption::default(),
                    })]),
                ),
            );

            let mut anonymous = shallow_section_with_content(
                "anon",
                "Anonymous",
                HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                    url: "/leaf".to_string(),
                    title: None,
                    option: SectionOption::default(),
                })]),
            );
            anonymous.metadata.0.insert(
                KEY_INTERNAL_ANON_SUBTREE.to_string(),
                HTMLContent::Plain("true".to_string()),
            );
            shallows.insert(Slug::new("anon"), anonymous);
            shallows.insert(Slug::new("leaf"), shallow_section("leaf", "Leaf"));

            let state = compile_all(&shallows).unwrap();
            let root = state.compiled().get(&Slug::new("index")).unwrap();
            let (html, _) = Writer::html_doc(root, &state).unwrap();

            let leaf_href = environment::full_html_url(Slug::new("leaf"));
            let anon_href = environment::full_html_url(Slug::new("anon"));
            let anon_hash_href = format!("#{}", crate::slug::to_hash_id("anon"));
            assert!(html.contains(&format!(r#"href="{}""#, leaf_href)));
            assert!(html.contains(&format!(r#"href="{}""#, anon_hash_href)));
            assert!(!html.contains(&format!(r#"href="{}""#, anon_href)));
        });
    }
}
