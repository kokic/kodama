mod base36;
mod config;
mod entry;
mod handler;
mod html_flake;
mod html_macro;
mod recorder;
mod typst_cli;

use clap::Parser;
use config::{dir_config, input_path, join_path, output_path, parent_dir};
use entry::{EntryMetaData, HtmlEntry};
use handler::Handler;
use html_flake::{html_doc, html_section, html_toc_block};
use pulldown_cmark::{html, CowStr, Event, Options};
use pulldown_cmark_to_cmark::cmark;
use recorder::{Context, Recorder};
use std::collections::HashMap;

fn to_slug(fullname: &str) -> String {
    let slug = &fullname[0..fullname.rfind('.').unwrap_or(fullname.len())];
    slug.replace("\\", "/")
}

pub fn adjust_name(path: &str, expect: &str, target: &str) -> String {
    let prefix = if path.ends_with(expect) {
        &path[0..path.len() - expect.len()]
    } else {
        path
    };
    format!("{}{}", prefix, target)
}

pub fn prepare_recorder(
    relative_dir: &str,
    filename: &str,
) -> (
    String,
    HashMap<std::string::String, std::string::String>,
    Recorder,
) {
    // global data store
    let mut metadata: HashMap<String, String> = HashMap::new();
    let fullname = join_path(relative_dir, filename);
    metadata.insert("slug".to_string(), to_slug(&fullname));

    // local contents recorder
    let recorder = Recorder::new(relative_dir);

    let markdown_path = input_path(&fullname);
    let expect = format!("file not found: {}", markdown_path);
    let markdown_input = std::fs::read_to_string(markdown_path).expect(&expect);

    return (markdown_input, metadata, recorder);
}

/// markdown + typst => markdown + svg + css
fn eliminate_typst(relative_dir: &str, filename: &str, holder: &mut String) {
    let (markdown_input, mut metadata, mut recorder) = prepare_recorder(relative_dir, filename);

    let mut handlers: Vec<Box<dyn Handler>> = vec![
        Box::new(handler::typst_image::TypstImage {}),
        Box::new(handler::katex_compat::KatexCompact {}),
    ];

    let parser = pulldown_cmark::Parser::new_ext(
        &markdown_input,
        Options::ENABLE_MATH.union(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS),
    );

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
                    html = html.or(handler.end(&tag, &mut recorder));
                }
                html.map(|s| event = Event::Html(CowStr::Boxed(s.into())));
            }

            Event::Text(s) => {
                handlers
                    .iter_mut()
                    .for_each(|handler| handler.text(s, &mut recorder));

                match recorder.context {
                    Context::Metadata if s.trim().len() != 0 => {
                        println!("Metadata: {:?}", s);
                        let pos = s.find(':').expect("metadata item expect `name: value`");
                        let key = s[0..pos].trim().to_string();
                        let val = s[pos + 1..].trim().to_string();
                        metadata.insert(key, val);
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

            Event::Html(_s) => { /* println!("Html: {:?}", s) */ }
            Event::InlineHtml(s) => println!("InlineHtml: {:?}", s),
            Event::Code(s) => println!("Code: {:?}", s),
            Event::FootnoteReference(s) => println!("FootnoteReference: {:?}", s),
            Event::TaskListMarker(b) => println!("TaskListMarker: {:?}", b),
            Event::SoftBreak => { /* println!("SoftBreak") */ }
            Event::HardBreak => println!("HardBreak"),
            Event::Rule => println!("Rule"),
        };

        match recorder.is_none() {
            true => Some(event),
            _ => None,
        }
    });

    cmark(parser, holder).unwrap();
}

/// parse markdown and generate HTML
fn parse_markdown(relative_dir: &str, filename: &str) -> HtmlEntry {
    let (markdown_input, mut metadata, mut recorder) = prepare_recorder(relative_dir, filename);

    let mut handlers: Vec<Box<dyn Handler>> = vec![
        Box::new(handler::embed_markdown::Embed {}),
        Box::new(handler::typst_image::TypstImage {}),
        Box::new(handler::katex_compat::KatexCompact {}),
    ];

    let parser = pulldown_cmark::Parser::new_ext(
        &markdown_input,
        Options::ENABLE_MATH.union(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS),
    );

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
                    html = html.or(handler.end(&tag, &mut recorder));
                }
                html.map(|s| event = Event::Html(CowStr::Boxed(s.into())));
            }

            Event::Text(s) => {
                handlers
                    .iter_mut()
                    .for_each(|handler| handler.text(s, &mut recorder));

                match recorder.context {
                    Context::Metadata if s.trim().len() != 0 => {
                        println!("Metadata: {:?}", s);
                        let pos = s.find(':').expect("metadata item expect `name: value`");
                        let key = s[0..pos].trim().to_string();
                        let val = s[pos + 1..].trim().to_string();
                        metadata.insert(key, val);
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

            Event::Html(_s) => { /* println!("Html: {:?}", s) */ }
            Event::InlineHtml(s) => println!("InlineHtml: {:?}", s),
            Event::Code(s) => println!("Code: {:?}", s),
            Event::FootnoteReference(s) => println!("FootnoteReference: {:?}", s),
            Event::TaskListMarker(b) => println!("TaskListMarker: {:?}", b),
            Event::SoftBreak => { /* println!("SoftBreak") */ }
            Event::HardBreak => println!("HardBreak"),
            Event::Rule => println!("Rule"),
        };

        match recorder.is_none() {
            true => Some(event),
            _ => None,
        }
    });

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    let metadata = EntryMetaData(metadata);
    let content = html_output;

    return HtmlEntry {
        metadata,
        content,
        catalog: recorder.catalog,
    };
}

pub fn html_article_inner(entry: &HtmlEntry, hide_metadata: bool) -> String {
    let metadata = &entry.metadata;
    let summary = metadata.to_header();
    let content = &entry.content;
    html_section(&summary, content, hide_metadata, metadata.texon())
}

fn write_html_content(filepath: &str, entry: &HtmlEntry) {
    let article_inner = html_article_inner(entry, false);
    let html = html_doc(&article_inner, &html_toc_block(&entry.catalog));
    let _ = std::fs::write(filepath, html);
}

fn write_and_inline_html_content(filepath: &str, entry: &HtmlEntry) -> String {
    let catalog = html_toc_block(&entry.catalog);
    let article_inner = html_article_inner(entry, false);
    let html = html_doc(&article_inner, &catalog);
    let _ = std::fs::write(filepath, html);
    // inline article content
    html_article_inner(entry, true)
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    // /// Creates new markdown file with name in the format "CAT-003S".
    // #[command(visible_alias = "n")]
    // New(NewCommand),
    /// Compiles an input markdown file into HTML format.
    #[command(visible_alias = "c")]
    Compile(CompileCommand),

    // /// Compiles an input markdown file into markdown and SVGs.
    #[command(visible_alias = "i")]
    Inline(CompileCommand),
}

#[derive(clap::Args)]
struct NewCommand {
    // Categorial name.
    category: String,
}

#[derive(clap::Args)]
struct CompileCommand {
    /// Path to input Typst file.
    input: String,

    /// Path to output dir.
    #[arg(short, long, default_value_t = format!("./publish"))]
    output: String,

    /// Configures the project root (for absolute paths)
    #[arg(short, long, default_value_t = format!("./"))]
    root: String,
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        // Command::New(new_command) => {
        //     let category = &new_command.category;
        //     let (parent, category) = parent_dir_create_all(&category);
        // },
        Command::Inline(compile_command) => {
            let input = compile_command.input.as_str();
            let output = compile_command.output.as_str();
            dir_config(&config::OUTPUT_DIR, output.to_string());

            let (root_dir, filename) = parent_dir(&input);
            dir_config(&config::ROOT_DIR, root_dir);

            let mut markdown = String::new();
            eliminate_typst("", &filename, &mut markdown);
            let filepath = output_path(&filename);
            let _ = std::fs::write(filepath, markdown);
        }
        Command::Compile(compile_command) => {
            let input = compile_command.input.as_str();
            let output = compile_command.output.as_str();
            dir_config(&config::OUTPUT_DIR, output.to_string());

            let (root_dir, filename) = parent_dir(&input);
            dir_config(&config::ROOT_DIR, root_dir);

            let entry = parse_markdown("", &filename);
            let filepath = output_path(&adjust_name(&filename, ".md", ".html"));
            write_html_content(&filepath, &entry);
        }
    }
}
