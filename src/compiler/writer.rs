use std::{ops::Not, path::Path};

use crate::{concepts::taxon::Taxon, config, entry::EntryMetaData, html, compiler::counter::Counter};

use super::section::{Section, SectionContent};

pub fn html_article_inner(
    metadata: &EntryMetaData,
    contents: &String,
    hide_metadata: bool,
    open: bool,
    adhoc_title: Option<&str>,
    adhoc_taxon: Option<&str>,
) -> String {
    let summary = metadata.to_header(adhoc_title, adhoc_taxon);

    let article_id = metadata.id();
    crate::html_flake::html_section(
        &summary,
        contents,
        hide_metadata,
        open,
        article_id,
        metadata.taxon_text(),
    )
}

pub struct Writer {}

impl Writer {
    pub fn write(section: &Section) {
        let html_url = format!("{}.html", section.slug());
        let filepath = crate::config::output_path(&html_url);

        let mut counter = Counter::init();

        let (article_inner, items) = Writer::section_to_html(section, &mut counter, true);
        let catalog_html = items
            .is_empty()
            .not()
            .then(|| Writer::catalog_block(&items))
            .unwrap_or_default();

        let html = crate::html_flake::html_doc(
            section.metadata.title().map_or("", |s| s),
            &article_inner,
            &catalog_html,
        );

        match std::fs::write(&filepath, html) {
            Ok(()) => {
                let title = section.metadata.title().map_or("", |s| s);
                let output_path = crate::slug::pretty_path(Path::new(&html_url));
                println!("Output: {:?} {}", title, output_path);
            }
            Err(err) => eprintln!("{:?}", err),
        }
    }

    fn catalog_block(items: &str) -> String {
        html!(div class = "block" =>
          (html!(h1 => "Table of Contents")) (items))
    }

    fn catalog_item(section: &Section, taxon: &str, child_html: &str) -> String {
        let slug = &section.slug();
        let text = section.metadata.title().unwrap();
        let slug_url = config::full_html_url(&slug);
        let title = format!("{} [{}]", text, slug);
        let href = format!("#{}", crate::slug::to_hash_id(slug)); // #id

        let mut class_name: Vec<String> = vec![];
        if !section.option.details_open {
            class_name.push("item-summary".to_string());
        }

        html!(li class = {class_name.join(" ")} =>
          (html!(a class = "bullet", href={slug_url}, title={title} => "■"))
          (html!(span class = "link" =>
            (html!(a href = {href} =>
              (html!(span class = "taxon" => {taxon}))
              (text)))))
          (child_html))
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
