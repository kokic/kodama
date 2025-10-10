// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::collections::{BTreeSet, HashMap, HashSet};

use eyre::OptionExt;

use crate::{
    entry::{EntryMetaData, HTMLMetaData, MetaData, KEY_SLUG, KEY_TITLE},
    environment,
    ordered_map::OrderedMap,
    path_utils,
    slug::{self, Slug},
};

use super::{
    callback::Callback,
    section::{HTMLContent, LazyContent, Section, SectionContent, SectionContents, ShallowSection},
    taxon::Taxon,
};

#[derive(Debug)]
pub struct CompileState {
    residued: BTreeSet<Slug>,
    compiled: HashMap<Slug, Section>,
    callback: Callback,
}

type Shallows = HashMap<Slug, ShallowSection>;

pub fn compile_all(mut shallows: Shallows) -> eyre::Result<CompileState> {
    for shallow in shallows.values_mut() {
        shallow.metadata.compute_textual_attrs();
    }

    let residued: BTreeSet<Slug> = shallows.keys().copied().collect();

    let mut state = CompileState::new(residued);
    state
        .compile(&shallows, Slug::new("index"))
        .ok_or_eyre("Missing `index` section, please provide `index.md` or `index.typst`")?;

    /*
     * Unlinked or unembedded pages.
     */
    while let Some(slug) = state.residued.pop_first() {
        state.compile(&shallows, slug);
    }

    Ok(state)
}

impl CompileState {
    fn new(residued: BTreeSet<Slug>) -> CompileState {
        CompileState {
            residued,
            compiled: HashMap::new(),
            callback: Callback::new(),
        }
    }

    fn compile(&mut self, shallows: &Shallows, slug: Slug) -> Option<&Section> {
        self.fetch_section(shallows, slug)
    }

    fn fetch_section(&mut self, shallows: &Shallows, slug: Slug) -> Option<&Section> {
        if self.compiled.contains_key(&slug) {
            Some(self.compiled.get(&slug).unwrap())
        } else {
            shallows
                .get(&slug)
                .map(|shallow| self.compile_shallow(shallows, shallow))
        }
    }

    fn compile_shallow(&mut self, shallows: &Shallows, spanned: &ShallowSection) -> &Section {
        let slug = spanned.slug();
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

                            let refered = match self.fetch_section(shallows, child_slug) {
                                Some(refered_section) => refered_section,
                                None => {
                                    eprintln!(
                                        "Error: [{}] attempting to fetch a non-existent [{}].",
                                        slug, child_slug,
                                    );
                                    continue;
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

                            if link_slug != slug && is_reference(shallows, link_slug) {
                                references.insert(link_slug);
                            }

                            /*
                             * Making oneself the content of a backlink should not be expected behavior.
                             */
                            if link_slug != slug && is_enable_backlinks(shallows, link_slug) {
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

        // compile metadata
        let mut metadata = EntryMetaData(OrderedMap::new());
        metadata.update(KEY_SLUG.to_string(), slug.to_string());
        spanned.metadata.keys().for_each(|key| {
            // Obviously, the slug must be a pure string, so we do not perform compilation.
            if key == KEY_SLUG {
                return;
            }
            let value = spanned.metadata.get(key).unwrap();
            let spanned: ShallowSection = Self::metadata_to_section(value, slug);
            let compiled = self.compile_shallow(shallows, &spanned);
            let html = compiled.spanned();
            metadata.update(key.to_string(), html);
        });

        // remove from `self.residued` after compiled.
        self.residued.remove(&slug);

        let section = Section::new(metadata, children, references);
        self.compiled.insert(slug, section);
        self.compiled.get(&slug).unwrap()
    }

    fn metadata_to_section(content: &HTMLContent, current_slug: Slug) -> ShallowSection {
        let mut metadata = OrderedMap::new();
        metadata.insert(
            KEY_SLUG.to_string(),
            HTMLContent::Plain(current_slug.to_string()),
        );

        ShallowSection {
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

fn get_metadata(shallows: &Shallows, slug: Slug) -> Option<&HTMLMetaData> {
    shallows.get(&slug).map(|s| &s.metadata)
}

fn is_enable_backlinks(shallows: &Shallows, slug: Slug) -> bool {
    shallows
        .get(&slug)
        .map(|s| s.metadata.is_enable_backlinks())
        .unwrap_or(true)
}

fn is_reference(shallows: &Shallows, slug: Slug) -> bool {
    shallows
        .get(&slug)
        .map(|s| {
            let metadata = &s.metadata;
            metadata.is_asref().unwrap_or(environment::asref())
                || Taxon::is_reference(metadata.data_taxon().map_or("", String::as_str))
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subsection_slug() {
        assert_eq!(subsection_slug(Slug::new("a/b"), "c/d.md"), "a/c/d");
        assert_eq!(subsection_slug(Slug::new("a/b"), "./c/d.md"), "a/c/d");

        assert_eq!(subsection_slug(Slug::new("a/b"), "/c/d.md"), "c/d");
    }
}
