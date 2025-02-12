use std::collections::{HashMap, HashSet};

use crate::{config, entry::EntryMetaData, slug};

use super::{
    callback::Callback,
    parser::parse_spanned_markdown,
    section::{HTMLContent, LazyContent, Section, SectionContent, SectionContents, ShallowSection},
    taxon::Taxon,
};

#[derive(Debug)]
pub struct CompileState {
    pub residued: HashMap<String, ShallowSection>,
    pub compiled: HashMap<String, Section>,
    pub metadata: HashMap<String, EntryMetaData>,
    pub callback: Callback,
}

impl CompileState {
    pub fn new() -> CompileState {
        CompileState {
            residued: HashMap::new(),
            compiled: HashMap::new(),
            metadata: HashMap::new(),
            callback: Callback::new(),
        }
    }

    pub fn compile(&mut self, slug: &str) -> &Section {
        self.fetch_section(slug)
    }

    pub fn compile_all(&mut self) {
        self.metadata = self
            .residued
            .iter()
            .map(|(key, value)| (key.to_string(), value.metadata.clone()))
            .collect();

        self.compile("index");
        /*
         * Unlinked or unembedded pages.
         */
        let residued_slugs: Vec<String> = self.residued.keys().map(|s| s.to_string()).collect();
        for slug in residued_slugs {
            self.compile(&slug);
        }
    }

    fn fetch_section(&mut self, slug: &str) -> &Section {
        if self.compiled.contains_key(slug) {
            return self.compiled.get(slug).unwrap();
        }

        if self.residued.contains_key(slug) {
            let shallow = self.residued.remove(slug).unwrap();
            return self.compile_shallow(shallow);
        }

        unreachable!("CompileState::fetch_section")
    }

    fn compile_shallow(&mut self, shallow: ShallowSection) -> &Section {
        let slug = shallow.slug();
        let mut metadata = shallow.metadata;
        let mut children: SectionContents = vec![];
        let mut references: HashSet<String> = HashSet::new();

        match &shallow.content {
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
                            let child_slug = slug::to_slug(&embed_content.url);
                            let refered = self.fetch_section(&child_slug);

                            if embed_content.option.details_open {
                                references.extend(refered.references.clone());
                            }
                            callback.insert_parent(child_slug, slug.to_string());

                            let mut child_section = refered.clone();
                            child_section.option = embed_content.option.clone();
                            if let Some(title) = &embed_content.title {
                                child_section
                                    .metadata
                                    .update("title".to_string(), title.to_string())
                            };
                            children.push(SectionContent::Embed(child_section));
                        }
                        LazyContent::Local(local_link) => {
                            let link_slug = &local_link.slug;
                            let article_title = self
                                .get_metadata(&link_slug, "page-title")
                                .or_else(|| self.get_metadata(&link_slug, "title"))
                                .map_or("", |s| s);
                            let article_taxon =
                                self.get_metadata(&link_slug, "taxon").map_or("", |s| s);

                            if Taxon::is_reference(&article_taxon) {
                                references.insert(link_slug.to_string());
                            }
                            callback.insert_backlinks(link_slug.to_string(), vec![slug.to_string()]);

                            let local_link = local_link.text.clone();
                            let text = local_link.unwrap_or(article_title.to_string());

                            let html = crate::html_flake::html_link(
                                &config::full_html_url(link_slug),
                                &format!("{} [{}]", article_title, link_slug),
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
        let metadata_keys: Vec<String> = metadata.enable_markdown_keys();
        metadata_keys.iter().for_each(|key| {
            let value = metadata.get(key).unwrap();
            let spanned = parse_spanned_markdown(value, &slug).unwrap();
            let compiled = self.compile_shallow(spanned);
            let html = compiled.spanned();
            metadata.update(key.to_string(), html);
        });

        // remove from `self.residued` after compiled.
        self.residued.remove(&slug);

        let section = Section::new(metadata, children, references);
        self.compiled.insert(slug.to_string(), section);
        self.compiled.get(&slug).unwrap()
    }

    pub fn get_metadata(&self, slug: &str, key: &str) -> Option<&String> {
        self.metadata.get(slug).map(|e| e.get(key)).flatten()
    }
}
