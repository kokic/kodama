use std::{collections::HashSet, ops::Not, path::Path};

use crate::{
    compiler::counter::Counter,
    config, html,
    html_flake::{self, html_article_inner},
};

use super::{
    section::{Section, SectionContent},
    state::CompileState,
    taxon::Taxon,
};

pub struct Writer {}

impl Writer {
    pub fn write(section: &Section, state: &CompileState) {
        let (html, page_title) = Writer::html_doc(section, state);
        let html_url = format!("{}.html", section.slug());
        let filepath = crate::config::output_path(&html_url);

        match std::fs::write(&filepath, html) {
            Ok(()) => {
                let output_path = crate::slug::pretty_path(Path::new(&html_url));
                println!("Output: {:?} {}", page_title, output_path);
            }
            Err(err) => eprintln!("{:?}", err),
        }
    }

    pub fn html_doc(section: &Section, state: &CompileState) -> (String, String) {
        let mut counter = Counter::init();

        let (article_inner, items) = Writer::section_to_html(section, &mut counter, true);
        let catalog_html = items
            .is_empty()
            .not()
            .then(|| Writer::catalog_block(&items))
            .unwrap_or_default();

        let html_header = Writer::header(state, &section.slug());
        let footer_html = Writer::footer(state, &section.references);
        let page_title = section
            .metadata
            .get("page-title")
            .map(|s| s.as_str())
            .unwrap_or_else(|| section.metadata.title().map_or("", |s| s));

        let html = crate::html_flake::html_doc(
            &page_title,
            &html_header,
            &article_inner,
            &footer_html,
            &catalog_html,
        );

        (html, page_title.to_string())
    }

    fn header(state: &CompileState, slug: &str) -> String {
        state
            .callback
            .get(slug)
            .and_then(|callback| {
                let parent = &callback.parent;
                state.compiled.get(parent).map(|section| {
                    let href = config::full_html_url(parent);
                    let title = section.metadata.title().map_or("", |s| s);
                    html_flake::html_header_nav(title, &href)
                })
            })
            .unwrap_or_default()
    }

    fn footer(state: &CompileState, references: &HashSet<String>) -> String {
        references
            .iter()
            .map(|slug| {
                // slug.to_string()
                let section = state.compiled.get(slug).unwrap();
                Writer::footer_section_to_html(section)
            })
            .reduce(|s, t| s + &t)
            .map(|s| html_flake::html_footer_section(&s))
            .unwrap_or_default()
    }

    fn catalog_block(items: &str) -> String {
        html!(div class = "block" =>
          (html!(h1 => "Table of Contents")) (items))
    }

    fn catalog_item(section: &Section, taxon: &str, child_html: &str) -> String {
        let slug = &section.slug();
        let text = section.metadata.title().unwrap();
        html_flake::catalog_item(slug, text, section.option.details_open, taxon, child_html)
    }

    fn footer_content_to_html(content: &SectionContent) -> String {
        match content {
            SectionContent::Plain(s) => s.to_string(),
            SectionContent::Embed(section) => Writer::footer_section_to_html(section),
        }
    }

    fn footer_section_to_html(section: &Section) -> String {
        let contents = match section.children.len() > 0 {
            false => String::new(),
            true => section
                .children
                .iter()
                .map(Writer::footer_content_to_html)
                .reduce(|s, t| s + &t)
                .unwrap(),
        };

        html_article_inner(&section.metadata, &contents, false, false, None, None)
    }

    pub fn section_to_html(
        section: &Section,
        counter: &mut Counter,
        toplevel: bool,
    ) -> (String, String) {
        let adhoc_taxon = Writer::taxon(section, counter);
        let (contents, items) = match section.children.len() > 0 {
            false => (String::new(), String::new()),
            true => {
                let mut subcounter = match section.option.numbering {
                    true => counter.left_shift(),
                    false => counter.clone(),
                };
                section
                    .children
                    .iter()
                    .map(|c| Writer::content_to_html(c, &mut subcounter))
                    .reduce(|s, t| (s.0 + &t.0, s.1 + &t.1))
                    .unwrap()
            }
        };

        let child_html = items
            .is_empty()
            .not()
            .then(|| format!(r#"<ul class="block">{}</ul>"#, &items))
            .unwrap_or_default();

        let catalog_item = match toplevel {
            true => child_html,
            false => section
                .option
                .catalog
                .then(|| Writer::catalog_item(section, &adhoc_taxon, &child_html))
                .unwrap_or(String::new()),
        };

        let article_inner = html_article_inner(
            &section.metadata,
            &contents,
            !toplevel,
            section.option.details_open,
            None,
            Some(adhoc_taxon.as_str()),
        );
        (article_inner, catalog_item)
    }

    fn content_to_html(content: &SectionContent, counter: &mut Counter) -> (String, String) {
        match content {
            SectionContent::Plain(s) => (s.to_string(), String::new()),
            SectionContent::Embed(section) => Writer::section_to_html(section, counter, false),
        }
    }

    fn taxon(section: &Section, counter: &mut Counter) -> String {
        if section.option.numbering {
            counter.step_mut();
            let numbering = Some(counter.display());
            let text = section.metadata.taxon_text().map_or("", |s| s);
            let taxon = Taxon::new(numbering, text.to_string());
            return taxon.display();
        }
        section.metadata.taxon_text().map_or("", |s| s).to_string()
    }
}
