use crate::{
    config::{self},
    entry,
    handler::{self, embed_markdown::write_to_html},
    html_flake, recorder, slug,
};

use config::input_path;
use entry::{EntryMetaData, HtmlEntry};
use handler::Handler;
use html_flake::html_section;
use pulldown_cmark::{html, CowStr, Event, Options, Tag, TagEnd};
use pulldown_cmark_to_cmark::cmark;
use recorder::{ParseRecorder, State};
use std::collections::HashMap;

pub fn prepare_container(
    filename: &str,
) -> Result<(String, HashMap<String, String>, ParseRecorder), CompileError> {
    // global data store
    let mut metadata: HashMap<String, String> = HashMap::new();
    let fullname = filename;
    metadata.insert("slug".to_string(), slug::to_slug(&fullname));

    // local contents recorder
    let recorder = ParseRecorder::new(fullname.to_string());
    let markdown_path = input_path(&fullname);
    match std::fs::read_to_string(&markdown_path) {
        Err(err) => Err(CompileError::FileNotFound(err, markdown_path)),
        Ok(markdown_input) => {
            return Ok((markdown_input, metadata, recorder));
        }
    }
}

const OPTIONS: Options = Options::ENABLE_MATH
    .union(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS)
    .union(Options::ENABLE_TABLES)
    .union(Options::ENABLE_SMART_PUNCTUATION)
    .union(Options::ENABLE_FOOTNOTES);

pub fn parse_content(
    markdown_input: &str,
    recorder: &mut ParseRecorder,
    metadata: &mut HashMap<String, String>,
    handlers: &mut Vec<Box<dyn Handler>>,
    history: &mut Vec<String>, 
    ignore_paragraph: bool,
) -> Result<String, CompileError> {
    let parser = pulldown_cmark::Parser::new_ext(&markdown_input, OPTIONS);
    let parser = parser.filter_map(|mut event| {
        match &event {
            Event::Start(tag) => {
                if ignore_paragraph {
                    match tag {
                        Tag::Paragraph => return None,
                        _ => (),
                    }
                }
                handlers
                    .iter_mut()
                    .for_each(|handler| handler.start(&tag, recorder));
            }

            Event::End(tag) => {
                if ignore_paragraph {
                    match tag {
                        TagEnd::Paragraph => return None,
                        _ => (),
                    }
                }
                let mut html: Option<String> = None;
                for handler in handlers.iter_mut() {
                    html = html.or(handler.end(&tag, recorder, history));
                }
                html.map(|s| event = Event::Html(CowStr::Boxed(s.into())));
            }

            Event::Text(s) => {
                handlers
                    .iter_mut()
                    .for_each(|handler| handler.text(s, recorder, metadata, history));
            }

            Event::InlineMath(s) => {
                let mut html = String::new();
                handlers.iter_mut().for_each(|handler| {
                    handler.inline_math(&s, recorder).map(|s| html = s);
                });
                event = Event::Html(CowStr::Boxed(html.into()));
            }

            Event::DisplayMath(s) => {
                let mut html = String::new();
                handlers.iter_mut().for_each(|handler| {
                    handler.display_math(&s, recorder).map(|s| html = s);
                });
                event = Event::Html(CowStr::Boxed(html.into()));
            }

            Event::InlineHtml(s) => {
                handlers
                    .iter_mut()
                    .for_each(|handler| handler.inline_html(s, recorder, metadata));
            }

            _ => (),
        };

        match recorder.is_html_writable() {
            true => Some(event),
            _ => None,
        }
    });

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    return Ok(html_output);
}

/// parse markdown and generate HTML
pub fn parse_markdown(filename: &str, history: &mut Vec<String>) -> Result<HtmlEntry, CompileError> {
    let mut handlers: Vec<Box<dyn Handler>> = vec![
        Box::new(handler::figure::Figure),
        Box::new(handler::typst_image::TypstImage),
        Box::new(handler::katex_compat::KatexCompact),
        Box::new(handler::embed_markdown::Embed),
    ];

    let (markdown_input, mut metadata, mut recorder) = prepare_container(filename)?;
    let content = parse_content(
        &markdown_input,
        &mut recorder,
        &mut metadata,
        &mut handlers,
        history, 
        false,
    )?;
    let metadata = EntryMetaData(metadata);

    return Ok(HtmlEntry {
        metadata,
        content,
        catalog: recorder.catalog,
    });
}

pub fn html_article_inner(entry: &HtmlEntry, hide_metadata: bool, open: bool) -> String {
    let metadata = &entry.metadata;
    let summary = metadata.to_header();
    let content = &entry.content;
    let article_id = metadata.id();
    html_section(
        &summary,
        content,
        hide_metadata,
        open,
        article_id,
        metadata.taxon(),
    )
}

pub enum CompileError {
    FileNotFound(std::io::Error, String),
}

impl std::fmt::Debug for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FileNotFound(err, path) => f
                .debug_struct("FileNotFound")
                .field("err", err)
                .field("path", path)
                .finish(),
        }
    }
}

/**
 * `filename` - the workspace index / home file name. 
 */
pub fn compile_workspace(filename: &str) -> Result<HtmlEntry, CompileError> {
    let mut entire = config::history(); // read-only
    let result= compile_to_html(filename, &mut entire);
    
    /*
     * Update an entire vector to avoid frequent calls to `Mutex::lock`. 
     */
    let mut history = config::HISTORY.lock().unwrap();
    *history = entire;
    result
}

pub fn compile_to_html(
    filename: &str,
    history: &mut Vec<String>,
) -> Result<HtmlEntry, CompileError> {
    let html_url = adjust_name(&filename, ".md", ".html");
    /*
     * An improvement that can be implemented here is to store (in memory or on disk)
     * the `HtmlEntry` instances that have already been generated, and then reuse
     * these `HtmlEntry` instances when parsing files that have already been processed.
     *
     * Of course, in general, the likelihood of this scenario occurring is quite low.
     */
    let mut entry = parse_markdown(&filename, history)?;
    write_to_html(&html_url, &mut entry);

    history.push(filename.to_string());

    Ok(entry)
}

pub fn compile_links() {
    let linked = config::linked(); // read-only
    let history = config::history(); // read-only

    // drop all history from linked
    let linked: std::collections::HashSet<_> = linked.iter().collect();
    let history: std::collections::HashSet<_> = history.iter().collect();

    for blink in linked {
        let (source, linked_url) = (&blink.source, &blink.target);
        if !history.contains(&linked_url) {
            /*
             * Note: Here we no longer need to update the `history`.
             */
            match compile_to_html(&linked_url, &mut vec![]) {
                Err(err) => eprintln!("{:?} at {}", err, source),
                _ => (),
            }
        }
    }
}

pub fn adjust_name(path: &str, expect: &str, target: &str) -> String {
    let prefix = if path.ends_with(expect) {
        &path[0..path.len() - expect.len()]
    } else {
        path
    };
    format!("{}{}", prefix, target)
}

pub fn parse_spanned_markdown(
    markdown_input: &str,
    current: String,
    history: &mut Vec<String>,
) -> Result<String, CompileError> {
    let mut recorder = ParseRecorder::new(current);
    let mut metadata = HashMap::new();
    let mut handlers: Vec<Box<dyn Handler>> = vec![
        Box::new(handler::figure::Figure),
        Box::new(handler::typst_image::TypstImage),
        Box::new(handler::katex_compat::KatexCompact),
        Box::new(handler::embed_markdown::Embed),
    ];

    let html_output = parse_content(
        &markdown_input,
        &mut recorder,
        &mut metadata,
        &mut handlers,
        history, 
        true,
    )?;
    return Ok(html_output);
}

/// markdown + typst => markdown + svg + css
pub fn eliminate_typst(filename: &str, holder: &mut String) -> Result<(), CompileError> {
    let (markdown_input, mut metadata, mut recorder) = prepare_container(filename)?;

    let mut handlers: Vec<Box<dyn Handler>> = vec![
        Box::new(handler::typst_image::TypstImage {}),
        Box::new(handler::katex_compat::KatexCompact {}),
    ];

    let parser = pulldown_cmark::Parser::new_ext(&markdown_input, OPTIONS);

    let parser = parser.filter_map(|mut event| {
        match &event {
            Event::Start(tag) => {
                handlers
                    .iter_mut()
                    .for_each(|handler| handler.start(&tag, &mut recorder));
            }

            Event::End(tag) => {
                let mut html: Option<String> = None;
                for handler in handlers.iter_mut() {
                    html = html.or(handler.end(&tag, &mut recorder, &mut vec![]));
                }
                html.map(|s| event = Event::Html(CowStr::Boxed(s.into())));
            }

            Event::Text(s) => {
                handlers
                    .iter_mut()
                    .for_each(|handler| handler.text(s, &mut recorder, &mut metadata, &mut vec![]));

                match recorder.state {
                    State::Metadata if s.trim().len() != 0 => {
                        let pos = s.find(':').expect("metadata item expect `name: value`");
                        let key = s[0..pos].trim();
                        let val = s[pos + 1..].trim();
                        metadata.insert(key.to_string(), val.to_string());
                    }
                    _ => (),
                }
            }

            Event::InlineMath(s) => {
                let mut html = String::new();
                handlers.iter_mut().for_each(|handler| {
                    handler.inline_math(&s, &mut recorder).map(|s| html = s);
                });
                event = Event::Html(CowStr::Boxed(html.into()));
            }

            Event::DisplayMath(s) => {
                let mut html = String::new();
                handlers.iter_mut().for_each(|handler| {
                    handler.display_math(&s, &mut recorder).map(|s| html = s);
                });
                event = Event::Html(CowStr::Boxed(html.into()));
            }

            _ => (),
        };

        match recorder.is_html_writable() {
            true => Some(event),
            _ => None,
        }
    });

    cmark(parser, holder).unwrap();
    Ok(())
}
