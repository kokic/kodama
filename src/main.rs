mod base36;
mod config;
mod entry;
mod handler;
mod html_flake;
mod html_macro;
mod recorder;
mod slug;
mod typst_cli;

use clap::Parser;
use config::{dir_config, input_path, join_path, output_path, parent_dir};
use entry::{EntryMetaData, HtmlEntry};
use handler::{embed_markdown::write_to_html, Handler};
use html_flake::html_section;
use pulldown_cmark::{html, CowStr, Event, Options};
use pulldown_cmark_to_cmark::cmark;
use recorder::{Context, Recorder};
use std::collections::HashMap;

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
    metadata.insert("slug".to_string(), slug::to_slug(&fullname));

    // local contents recorder
    let recorder = Recorder::new(relative_dir);

    let markdown_path = input_path(&fullname);
    let expect = format!("file not found: {}", markdown_path);
    let markdown_input = std::fs::read_to_string(markdown_path).expect(&expect);

    return (markdown_input, metadata, recorder);
}

const OPTIONS: Options = Options::ENABLE_MATH
    .union(Options::ENABLE_YAML_STYLE_METADATA_BLOCKS)
    .union(Options::ENABLE_TABLES)
    .union(Options::ENABLE_SMART_PUNCTUATION);

/// markdown + typst => markdown + svg + css
fn eliminate_typst(relative_dir: &str, filename: &str, holder: &mut String) {
    let (markdown_input, mut metadata, mut recorder) = prepare_recorder(relative_dir, filename);

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
                    html = html.or(handler.end(&tag, &mut recorder));
                }
                html.map(|s| event = Event::Html(CowStr::Boxed(s.into())));
            }

            Event::Text(s) => {
                handlers
                    .iter_mut()
                    .for_each(|handler| handler.text(s, &mut recorder, &mut metadata));

                match recorder.context {
                    Context::Metadata if s.trim().len() != 0 => {
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

pub enum ParseInterrupt {
    Skiped,
    Fail,
}

impl ParseInterrupt {
    pub fn message(&self, info: Option<&str>) -> String {
        let info = info.map(|s| format!(": {}", s)).unwrap_or(".".to_string());
        match self {
            ParseInterrupt::Skiped => format!("Skip compilation of unmodified{}", info),
            ParseInterrupt::Fail => format!("Parse failed{}", info),
        }
    }
}

/// parse markdown and generate HTML
fn parse_markdown(relative_dir: &str, filename: &str) -> HtmlEntry {
    let (markdown_input, mut metadata, mut recorder) = prepare_recorder(relative_dir, filename);

    let mut handlers: Vec<Box<dyn Handler>> = vec![
        Box::new(handler::embed_markdown::Embed {}),
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
                    html = html.or(handler.end(&tag, &mut recorder));
                }
                html.map(|s| event = Event::Html(CowStr::Boxed(s.into())));
            }

            Event::Text(s) => {
                handlers
                    .iter_mut()
                    .for_each(|handler| handler.text(s, &mut recorder, &mut metadata));
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
            Event::InlineHtml(_s) => { /*println!("InlineHtml: {:?}", s)*/ }
            Event::Code(_s) => { /* println!("Code: {:?}", s) */ }
            Event::FootnoteReference(_s) => { /* println!("FootnoteReference: {:?}", s) */ }
            Event::TaskListMarker(_b) => { /* println!("TaskListMarker: {:?}", b) */ }
            Event::SoftBreak => { /* println!("SoftBreak") */ }
            Event::HardBreak => { /* println!("HardBreak") */ }
            Event::Rule => { /* println!("Rule") */ }
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

pub fn html_article_inner(
    entry: &HtmlEntry,
    hide_metadata: bool,
    open: bool,
) -> String {
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

    /// Compiles an input markdown file into markdown and SVGs.
    #[command(visible_alias = "i")]
    Inline(CompileCommand),

    /// Clean all markdown entry caches.
    Clean(CleanCommand),
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

#[derive(clap::Args)]
struct CleanCommand {
    // target: String,
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

            let (parent, filename) = parent_dir(&input);
            dir_config(&config::ROOT_DIR, compile_command.root.to_string());

            let mut markdown = String::new();
            eliminate_typst(&parent, &filename, &mut markdown);
            let filepath = output_path(&filename);
            let _ = std::fs::write(filepath, markdown);
        }
        Command::Compile(compile_command) => {
            let input = compile_command.input.as_str();
            let output = compile_command.output.as_str();
            dir_config(&config::OUTPUT_DIR, output.to_string());

            let (parent, filename) = parent_dir(&input);
            dir_config(&config::ROOT_DIR, compile_command.root.to_string());

            let entry = parse_markdown(&parent, &filename);
            let html_path = adjust_name(&filename, ".md", ".html");
            let html_path = output_path(&html_path);
            write_to_html(&html_path, &entry);
        }
        Command::Clean(clean_command) => {
            dir_config(&config::ROOT_DIR, clean_command.root.to_string());
            let _ = config::delete_all_html_cache();
        }
    }
}
