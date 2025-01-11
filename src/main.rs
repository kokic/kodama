mod kodama;
mod base36;
mod config;
mod entry;
mod handler;
mod html_flake;
mod html_macro;
mod recorder;
mod slug;
mod typst_cli;
mod traverse;

use clap::Parser;
use config::{dir_config, output_path};
use kodama::{adjust_name, compile_to_html, eliminate_typst};

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

            let filename = input;
            // let (parent, filename) = parent_dir(&input);
            dir_config(&config::ROOT_DIR, compile_command.root.to_string());

            let mut markdown = String::new();
            eliminate_typst(&filename, &mut markdown);
            let filepath = output_path(&filename);
            let _ = std::fs::write(filepath, markdown);
        }
        Command::Compile(compile_command) => {
            let input = compile_command.input.as_str();
            let output = compile_command.output.as_str();
            dir_config(&config::OUTPUT_DIR, output.to_string());
            dir_config(&config::ROOT_DIR, compile_command.root.to_string());
            // let (parent, filename) = parent_dir(&input);
            compile_to_html(input);
        }
        Command::Clean(clean_command) => {
            dir_config(&config::ROOT_DIR, clean_command.root.to_string());
            let _ = config::delete_all_html_cache();
        }
    }
}
