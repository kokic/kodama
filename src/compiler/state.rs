use std::collections::HashMap;

use crate::{config, slug};

use super::{
    parser::parse_spanned_markdown,
    section::{HTMLContent, LazyContent, Section, SectionContent, SectionContents, ShallowSection},
};

#[derive(Debug)]
pub struct CompileState {
    pub residued: Box<HashMap<String, ShallowSection>>,
    pub compiled: Box<HashMap<String, Section>>,
}

impl CompileState {
    pub fn new() -> CompileState {
        CompileState {
            residued: Box::new(HashMap::new()),
            compiled: Box::new(HashMap::new()),
        }
    }

    pub fn compile(&mut self, slug: &str) -> &Section {
        self.fetch_section(slug)
    }

    pub fn compile_all(&mut self) {
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

        unreachable!()
    }

    fn compile_shallow(&mut self, shallow: ShallowSection) -> &Section {
        let slug = shallow.slug();
        let mut metadata = shallow.metadata;
        let mut children: SectionContents = vec![];

        match &shallow.content {
            HTMLContent::Plain(html) => {
                children.push(SectionContent::Plain(html.to_string()));
            }
            HTMLContent::Lazy(lazy_contents) => {
                for lazy_content in lazy_contents {
                    match lazy_content {
                        LazyContent::Plain(html) => {
                            children.push(SectionContent::Plain(html.to_string()));
                        }
                        LazyContent::Embed(embed_content) => {
                            let slug = slug::to_slug(&embed_content.url);
                            let mut child_section = self.fetch_section(&slug).clone();
                            child_section.option = embed_content.option.clone();
                            if let Some(title) = &embed_content.title {
                                child_section
                                    .metadata
                                    .update("title".to_string(), title.to_string())
                            };

                            children.push(SectionContent::Embed(child_section));
                        }
                        LazyContent::Local(local_link) => {
                            let slug = &local_link.slug;
                            let article_title =
                                self.get_metadata(slug, "title").unwrap_or(slug);

                            let local_link = local_link.text.clone();
                            let text = local_link.unwrap_or(article_title.to_string());

                            let html = crate::html_flake::html_link(
                                &config::full_html_url(slug),
                                &format!("{} [{}]", article_title, slug),
                                &text,
                                crate::recorder::State::LocalLink.strify(),
                            );
                            children.push(SectionContent::Plain(html.to_string()));
                        }
                    }
                }
            }
        };

        let etc = metadata.etc_keys();
        if etc.len() > 0 {
            etc.iter().for_each(|key| {
                let value = metadata.get(key).unwrap();
                let spanned = parse_spanned_markdown(value, &slug).unwrap();
                let compiled = self.compile_shallow(spanned);
                let html = compiled.spanned();
                metadata.update(key.to_string(), html);
            });
        }

        let section = Section::new(metadata, children);
        self.compiled.insert(slug.to_string(), section);
        self.compiled.get(&slug).unwrap()
    }

    pub fn get_metadata(&self, slug: &str, key: &str) -> Option<&String> {
        self.residued
            .get(slug)
            .map(|s| s.metadata.get(key))
            .or(self.compiled.get(slug).map(|s| s.metadata.get(key)))
            .flatten()
    }
}
