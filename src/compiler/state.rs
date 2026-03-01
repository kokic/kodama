// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use eyre::eyre;
use std::collections::{BTreeSet, HashMap, HashSet};

use crate::{
    entry::{
        is_plain_metadata, EntryMetaData, HTMLMetaData, MetaData, KEY_EXT,
        KEY_INTERNAL_ANON_SUBTREE, KEY_SLUG, KEY_TITLE,
    },
    environment,
    ordered_map::OrderedMap,
    path_utils,
    slug::{self, Slug},
};

use super::{
    callback::{Callback, CallbackValue},
    section::{
        HTMLContent, LazyContent, Section, SectionContent, SectionContents, UnresolvedSection,
    },
    taxon::Taxon,
};

#[derive(Debug)]
pub struct CompileState {
    residued: BTreeSet<Slug>,
    compiled: HashMap<Slug, Section>,
    callback: Callback,
    visiting: HashSet<Slug>,
    compile_stack: Vec<Slug>,
}

type UnresolvedSections = HashMap<Slug, UnresolvedSection>;

pub fn compile_all(shallows: &UnresolvedSections) -> eyre::Result<CompileState> {
    compile_all_with_missing_index_warning(shallows, true)
}

pub fn compile_all_without_missing_index_warning(
    shallows: &UnresolvedSections,
) -> eyre::Result<CompileState> {
    compile_all_with_missing_index_warning(shallows, false)
}

fn compile_all_with_missing_index_warning(
    shallows: &UnresolvedSections,
    emit_missing_index_warning: bool,
) -> eyre::Result<CompileState> {
    let residued: BTreeSet<Slug> = shallows.keys().copied().collect();

    let mut state = CompileState::new(residued);
    if emit_missing_index_warning && state.compile(shallows, Slug::new("index"))?.is_none() {
        color_print::ceprintln!(
            "<y>Warning: Missing `index` section, please provide `index.md` or `index.typst`.</>"
        );
    } else if !emit_missing_index_warning {
        let _ = state.compile(shallows, Slug::new("index"))?;
    }

    /*
     * Unlinked or unembedded pages.
     */
    while let Some(slug) = state.residued.pop_first() {
        state.compile(shallows, slug)?;
    }

    state.normalize_internal_anonymous_graph();
    Ok(state)
}

impl CompileState {
    fn new(residued: BTreeSet<Slug>) -> CompileState {
        CompileState {
            residued,
            compiled: HashMap::new(),
            callback: Callback::new(),
            visiting: HashSet::new(),
            compile_stack: Vec::new(),
        }
    }

    fn compile(
        &mut self,
        shallows: &UnresolvedSections,
        slug: Slug,
    ) -> eyre::Result<Option<&Section>> {
        self.fetch_section(shallows, slug)
    }

    fn fetch_section(
        &mut self,
        shallows: &UnresolvedSections,
        slug: Slug,
    ) -> eyre::Result<Option<&Section>> {
        if self.compiled.contains_key(&slug) {
            return Ok(self.compiled.get(&slug));
        }

        if self.visiting.contains(&slug) {
            let mut chain: Vec<String> =
                self.compile_stack.iter().map(ToString::to_string).collect();
            chain.push(slug.to_string());
            return Err(eyre!("cyclic embed detected: {}", chain.join(" -> ")));
        }

        let Some(shallow) = shallows.get(&slug) else {
            return Ok(None);
        };
        self.visiting.insert(slug);
        self.compile_stack.push(slug);
        let result = self.compile_unresolved(shallows, shallow);
        self.compile_stack.pop();
        self.visiting.remove(&slug);
        result?;
        Ok(self.compiled.get(&slug))
    }

    fn compile_unresolved(
        &mut self,
        shallows: &UnresolvedSections,
        spanned: &UnresolvedSection,
    ) -> eyre::Result<()> {
        let slug = spanned.slug()?;
        let ext = spanned.ext()?;
        let mut children: SectionContents = vec![];
        let mut references: HashSet<Slug> = HashSet::new();

        match &spanned.content {
            HTMLContent::Plain(html) => {
                children.push(SectionContent::Plain(html.to_string()));
            }
            HTMLContent::Lazy(lazy_contents) => {
                let mut callback: Callback = Callback::new();

                for lazy_content in lazy_contents {
                    match lazy_content {
                        LazyContent::Plain(html) => {
                            children.push(SectionContent::Plain(html.to_string()));
                        }
                        LazyContent::Embed(embed_content) => {
                            let child_slug = subsection_slug(slug, &embed_content.url);

                            let refered = match self.fetch_section(shallows, child_slug)? {
                                Some(refered_section) => refered_section,
                                None => {
                                    return Err(eyre!(
                                        "[{}] attempting to fetch a non-existent [{}]",
                                        slug,
                                        child_slug
                                    ));
                                }
                            };

                            if embed_content.option.details_open {
                                references.extend(refered.references.clone());
                            }
                            callback.insert_parent(child_slug, slug);

                            let mut child_section = refered.clone();
                            child_section.option = embed_content.option.clone();
                            if let Some(title) = &embed_content.title {
                                child_section
                                    .metadata
                                    .update(KEY_TITLE.to_owned(), title.to_string())
                            };
                            children.push(SectionContent::Embed(child_section));
                        }
                        LazyContent::Local(local_link) => {
                            let link_slug = subsection_slug(slug, &local_link.url);

                            let metadata = get_metadata(shallows, link_slug);
                            let article_title_html = metadata
                                .and_then(|s| s.title())
                                .map_or_else(String::new, html_content_to_html_string);
                            let article_title_plain = metadata
                                .and_then(|s| s.title())
                                .map_or_else(String::new, HTMLContent::remove_all_tags);
                            let page_title_plain = metadata
                                .and_then(|s| s.page_title())
                                .map(|s| strip_html_tags(s))
                                .unwrap_or_else(|| article_title_plain.clone());

                            if link_slug != slug && is_reference(shallows, link_slug)? {
                                references.insert(link_slug);
                            }

                            /*
                             * Making oneself the content of a backlink should not be expected behavior.
                             */
                            if link_slug != slug
                                && backlinks_enabled(shallows, link_slug)?
                                && is_backlink(shallows, slug)?
                            {
                                callback.insert_backlinks(link_slug, vec![slug]);
                            }

                            let text = local_link.text.clone().unwrap_or(article_title_html);

                            let html = crate::html_flake::html_link(
                                &environment::full_html_url(link_slug),
                                &format!("{} [{}]", page_title_plain, link_slug),
                                &text,
                                crate::recorder::State::LocalLink.strify(),
                            );
                            children.push(SectionContent::Plain(html.to_string()));
                        }
                    }
                }

                self.callback.merge(callback);
            }
        };

        if let Some(parent) = spanned.metadata.parent() {
            self.callback.specify_parent(slug, parent);
        }

        // compile metadata
        let mut metadata = EntryMetaData(OrderedMap::new());
        for key in spanned.metadata.keys() {
            let Some(value) = spanned.metadata.get(key) else {
                return Err(eyre!(
                    "metadata key `{}` vanished while compiling `{}`",
                    key,
                    slug
                ));
            };
            if is_plain_metadata(key) {
                if let Some(val) = value.as_string() {
                    metadata.update(key.to_string(), val.to_owned());
                } else {
                    return Err(eyre!(
                        "metadata field `{}` in `{}` is expected to be plain text",
                        key,
                        slug
                    ));
                }
            } else {
                let spanned: UnresolvedSection = Self::metadata_to_section(value, slug, ext);
                self.compile_unresolved(shallows, &spanned)?;
                let compiled = self.compiled.get(&slug).ok_or_else(|| {
                    eyre!(
                        "compiled section `{}` disappeared while compiling metadata",
                        slug
                    )
                })?;
                let html = compiled.spanned();
                metadata.update(key.to_string(), html);
            };
        }

        // remove from `self.residued` after compiled.
        self.residued.remove(&slug);

        let section = Section::new(metadata, children, references);
        self.compiled.insert(slug, section);
        Ok(())
    }

    fn metadata_to_section(
        content: &HTMLContent,
        current_slug: Slug,
        current_ext: &str,
    ) -> UnresolvedSection {
        let mut metadata = OrderedMap::new();
        metadata.insert(
            KEY_SLUG.to_string(),
            HTMLContent::Plain(current_slug.to_string()),
        );
        metadata.insert(
            KEY_EXT.to_string(),
            HTMLContent::Plain(current_ext.to_string()),
        );

        UnresolvedSection {
            metadata: HTMLMetaData(metadata),
            content: content.clone(),
        }
    }

    pub fn compiled(&self) -> &HashMap<Slug, Section> {
        &self.compiled
    }

    pub fn callback(&self) -> &Callback {
        &self.callback
    }

    fn normalize_internal_anonymous_graph(&mut self) {
        let internal_slugs = self.collect_internal_anonymous_slugs();
        if internal_slugs.is_empty() {
            return;
        }

        for section in self.compiled.values_mut() {
            section
                .references
                .retain(|reference| !internal_slugs.contains(reference));
        }

        let normalized_parents: HashMap<Slug, Slug> = self
            .callback
            .0
            .iter()
            .map(|(&slug, value)| {
                (
                    slug,
                    Self::resolve_visible_parent(value.parent, &self.callback.0, &internal_slugs),
                )
            })
            .collect();

        for (&slug, value) in &mut self.callback.0 {
            value
                .backlinks
                .retain(|backlink| !internal_slugs.contains(backlink));
            if let Some(parent) = normalized_parents.get(&slug) {
                value.parent = *parent;
            }
        }

        self.callback
            .0
            .retain(|slug, _| !internal_slugs.contains(slug));
        self.compiled
            .retain(|slug, _| !internal_slugs.contains(slug));
    }

    fn collect_internal_anonymous_slugs(&self) -> HashSet<Slug> {
        self.compiled
            .iter()
            .filter_map(|(&slug, section)| {
                section
                    .metadata
                    .get_str(KEY_INTERNAL_ANON_SUBTREE)
                    .is_some_and(|value| value == "true")
                    .then_some(slug)
            })
            .collect()
    }

    fn resolve_visible_parent(
        mut parent: Slug,
        callbacks: &HashMap<Slug, CallbackValue>,
        internal_slugs: &HashSet<Slug>,
    ) -> Slug {
        let mut visited = HashSet::new();
        while internal_slugs.contains(&parent) {
            if !visited.insert(parent) {
                color_print::ceprintln!(
                    "<y>Warning: cyclic internal parent chain detected at `{}`; falling back to `index`.</>",
                    parent
                );
                return Slug::new("index");
            }
            parent = callbacks
                .get(&parent)
                .map_or(Slug::new("index"), |value| value.parent);
        }
        parent
    }
}

/// Calculate the slug of a subsection referenced by the current file, from the `url` referencing
/// it. If the url starts with `/`, the slug is considered absolute starting from the base of the
/// tree. Otherwise it's attached to the directory containing the current file.
fn subsection_slug(current_slug: Slug, url: &str) -> Slug {
    slug::to_slug(path_utils::relative_to_current(current_slug.as_str(), url))
}

fn get_metadata(shallows: &UnresolvedSections, slug: Slug) -> Option<&HTMLMetaData> {
    shallows.get(&slug).map(|s| &s.metadata)
}

fn html_content_to_html_string(content: &HTMLContent) -> String {
    content
        .as_string()
        .cloned()
        .unwrap_or_else(|| content.remove_all_tags())
}

fn strip_html_tags(text: &str) -> String {
    HTMLContent::Plain(text.to_string()).remove_all_tags()
}

fn backlinks_enabled(shallows: &UnresolvedSections, slug: Slug) -> eyre::Result<bool> {
    match shallows.get(&slug) {
        Some(section) => section.metadata.backlinks_enabled(),
        None => Ok(true),
    }
}

fn is_reference(shallows: &UnresolvedSections, slug: Slug) -> eyre::Result<bool> {
    match shallows.get(&slug) {
        Some(section) => {
            let metadata = &section.metadata;
            Ok(metadata.is_asref()?.unwrap_or(environment::asref())
                || Taxon::is_reference(metadata.data_taxon().map_or("", String::as_str)))
        }
        None => Ok(false),
    }
}

fn is_backlink(shallows: &UnresolvedSections, slug: Slug) -> eyre::Result<bool> {
    match shallows.get(&slug) {
        Some(section) => {
            let metadata = &section.metadata;
            Ok(metadata.is_asback()?.unwrap_or(true))
        }
        None => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::super::section::{EmbedContent, LocalLink, SectionOption};
    use super::*;
    use crate::{
        entry::{KEY_ASREF, KEY_INTERNAL_ANON_SUBTREE},
        ordered_map::OrderedMap,
    };

    fn shallow_with_content(slug: &str, content: HTMLContent) -> UnresolvedSection {
        let mut metadata = OrderedMap::new();
        metadata.insert(KEY_SLUG.to_string(), HTMLContent::Plain(slug.to_string()));
        metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain("md".to_string()));

        UnresolvedSection {
            metadata: HTMLMetaData(metadata),
            content,
        }
    }

    #[test]
    fn test_subsection_slug() {
        assert_eq!(subsection_slug(Slug::new("a/b"), "c/d.md"), "a/c/d");
        assert_eq!(subsection_slug(Slug::new("a/b"), "./c/d.md"), "a/c/d");
        assert_eq!(subsection_slug(Slug::new("index"), "./a.b"), "a.b");

        assert_eq!(subsection_slug(Slug::new("a/b"), "/c/d.md"), "c/d");
    }

    #[test]
    fn test_compile_all_returns_error_for_cyclic_embed() {
        let embed_to_b = LazyContent::Embed(EmbedContent {
            url: "/b.md".to_string(),
            title: None,
            option: SectionOption::default(),
        });
        let embed_to_a = LazyContent::Embed(EmbedContent {
            url: "/a.md".to_string(),
            title: None,
            option: SectionOption::default(),
        });

        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("a"),
            shallow_with_content("a", HTMLContent::Lazy(vec![embed_to_b])),
        );
        shallows.insert(
            Slug::new("b"),
            shallow_with_content("b", HTMLContent::Lazy(vec![embed_to_a])),
        );

        let err = compile_all(&shallows).unwrap_err();
        assert!(err.to_string().contains("cyclic embed"));
    }

    #[test]
    fn test_local_link_title_attribute_uses_plain_text() {
        let mut shallows = HashMap::new();

        shallows.insert(
            Slug::new("index"),
            shallow_with_content(
                "index",
                HTMLContent::Lazy(vec![LazyContent::Local(LocalLink {
                    url: "/target".to_string(),
                    text: None,
                })]),
            ),
        );

        let mut target_metadata = OrderedMap::new();
        target_metadata.insert(
            KEY_SLUG.to_string(),
            HTMLContent::Plain("target".to_string()),
        );
        target_metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain("md".to_string()));
        target_metadata.insert(
            KEY_TITLE.to_string(),
            HTMLContent::Plain(r#"<span lang="zh">abc</span>"#.to_string()),
        );
        target_metadata.insert(
            crate::entry::KEY_PAGE_TITLE.to_string(),
            HTMLContent::Plain(r#"<span lang="zh">abc</span>"#.to_string()),
        );
        shallows.insert(
            Slug::new("target"),
            UnresolvedSection {
                metadata: HTMLMetaData(target_metadata),
                content: HTMLContent::Plain(String::new()),
            },
        );

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();
        let html = state
            .compiled()
            .get(&Slug::new("index"))
            .and_then(|section| section.children.first())
            .and_then(|child| match child {
                SectionContent::Plain(html) => Some(html.as_str()),
                _ => None,
            })
            .expect("compiled index html");

        assert!(html.contains(r#"title="abc [target]""#));
        assert!(!html.contains("&lt;span"));
        assert!(html.contains(r#"><span lang="zh">abc</span></a>"#));
    }

    #[test]
    fn test_compile_filters_internal_anonymous_sections_from_compiled_graph() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("index"),
            shallow_with_content(
                "index",
                HTMLContent::Lazy(vec![LazyContent::Local(LocalLink {
                    url: "/anon".to_string(),
                    text: None,
                })]),
            ),
        );

        let mut anon = shallow_with_content("anon", HTMLContent::Plain("<p>anon</p>".to_string()));
        anon.metadata.0.insert(
            KEY_INTERNAL_ANON_SUBTREE.to_string(),
            HTMLContent::Plain("true".to_string()),
        );
        anon.metadata.0.insert(
            KEY_ASREF.to_string(),
            HTMLContent::Plain("true".to_string()),
        );
        shallows.insert(Slug::new("anon"), anon);

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();
        let index = state.compiled().get(&Slug::new("index")).unwrap();
        assert!(!index.references.contains(&Slug::new("anon")));
        assert!(!state.compiled().contains_key(&Slug::new("anon")));
        assert!(!state.callback().0.contains_key(&Slug::new("anon")));
    }

    #[test]
    fn test_compile_filters_internal_anonymous_backlinks_from_targets() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("index"),
            shallow_with_content(
                "index",
                HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                    url: "/anon".to_string(),
                    title: None,
                    option: SectionOption::default(),
                })]),
            ),
        );

        let mut anon = shallow_with_content(
            "anon",
            HTMLContent::Lazy(vec![LazyContent::Local(LocalLink {
                url: "/target".to_string(),
                text: None,
            })]),
        );
        anon.metadata.0.insert(
            KEY_INTERNAL_ANON_SUBTREE.to_string(),
            HTMLContent::Plain("true".to_string()),
        );
        shallows.insert(Slug::new("anon"), anon);
        shallows.insert(
            Slug::new("target"),
            shallow_with_content("target", HTMLContent::Plain("<p>target</p>".to_string())),
        );

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();
        let maybe_target_callback = state.callback().0.get(&Slug::new("target"));
        assert!(maybe_target_callback
            .map(|value| value.backlinks.is_empty())
            .unwrap_or(true));
    }

    #[test]
    fn test_compile_collapses_internal_parent_chain_to_visible_parent() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("index"),
            shallow_with_content(
                "index",
                HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                    url: "/anon".to_string(),
                    title: None,
                    option: SectionOption::default(),
                })]),
            ),
        );
        let mut anon = shallow_with_content(
            "anon",
            HTMLContent::Lazy(vec![LazyContent::Embed(EmbedContent {
                url: "/child".to_string(),
                title: None,
                option: SectionOption::default(),
            })]),
        );
        anon.metadata.0.insert(
            KEY_INTERNAL_ANON_SUBTREE.to_string(),
            HTMLContent::Plain("true".to_string()),
        );
        shallows.insert(Slug::new("anon"), anon);
        shallows.insert(
            Slug::new("child"),
            shallow_with_content("child", HTMLContent::Plain("<p>child</p>".to_string())),
        );

        let state = compile_all_without_missing_index_warning(&shallows).unwrap();
        let child_callback = state
            .callback()
            .0
            .get(&Slug::new("child"))
            .expect("child callback");
        assert_eq!(child_callback.parent, Slug::new("index"));
        assert!(!state.callback().0.contains_key(&Slug::new("anon")));
    }
}
