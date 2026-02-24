// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use eyre::eyre;
use std::collections::{BTreeSet, HashMap, HashSet};

use crate::{
    entry::{
        is_plain_metadata, EntryMetaData, HTMLMetaData, MetaData, KEY_EXT, KEY_SLUG, KEY_TITLE,
    },
    environment,
    ordered_map::OrderedMap,
    path_utils,
    slug::{self, Slug},
};

use super::{
    callback::Callback,
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
    let residued: BTreeSet<Slug> = shallows.keys().copied().collect();

    let mut state = CompileState::new(residued);
    if state.compile(shallows, Slug::new("index"))?.is_none() {
        color_print::ceprintln!(
            "<y>Warning: Missing `index` section, please provide `index.md` or `index.typst`.</>"
        );
    }

    /*
     * Unlinked or unembedded pages.
     */
    while let Some(slug) = state.residued.pop_first() {
        state.compile(shallows, slug)?;
    }

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
                            let article_title = get_metadata(shallows, link_slug).map_or("", |s| {
                                s.title().and_then(|c| c.as_string()).map_or("", |s| s)
                            });
                            let page_title = metadata
                                .map_or("", |s| s.page_title().map_or(article_title, |s| s));

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

                            let local_link = local_link.text.clone();
                            let text = local_link.unwrap_or(article_title.to_string());

                            let html = crate::html_flake::html_link(
                                &environment::full_html_url(link_slug),
                                &format!("{} [{}]", page_title, link_slug),
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
    use super::super::section::{EmbedContent, SectionOption};
    use super::*;
    use crate::ordered_map::OrderedMap;

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
}
