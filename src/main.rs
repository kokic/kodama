mod config;
mod entry;
mod handler;
mod html_flake;
mod html_macro;
mod kodama;
mod recorder;
mod slug;
mod typst_cli;

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
    /// Compiles an input markdown file into HTML format.
    #[command(visible_alias = "c")]
    Compile(CompileCommand),

    /// Compiles an input markdown file into markdown and SVGs.
    #[command(visible_alias = "i")]
    Inline(CompileCommand),

    /// Clean all build files (.cache & publish).
    Clean(CleanCommand),
}

#[derive(clap::Args)]
struct CompileCommand {
    /// Path to input Typst file.
    input: String,

    /// Base URL or publish URL (e.g. https://www.example.com/)
    #[arg(short, long, default_value_t = format!("/"))]
    base: String,

    /// Path to output dir.
    #[arg(short, long, default_value_t = format!("./publish"))]
    output: String,

    /// Configures the project root (for absolute paths)
    #[arg(short, long, default_value_t = format!("./"))]
    root: String,
}

#[derive(clap::Args)]
struct CleanCommand {
    // target: CleanTarget,
    /// Configures the project root (for absolute paths)
    #[arg(short, long, default_value_t = format!("./"))]
    root: String,
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Command::Inline(compile_command) => {
            let input = compile_command.input.as_str();
            let output = compile_command.output.as_str();
            dir_config(&config::OUTPUT_DIR, output.to_string());

            let filename = input;
            dir_config(&config::ROOT_DIR, compile_command.root.to_string());

            let mut markdown = String::new();
            let _ = eliminate_typst(&filename, &mut markdown);
            let filepath = output_path(&filename);
            let _ = std::fs::write(filepath, markdown);
        }
        Command::Compile(compile_command) => {
            let input = compile_command.input.as_str();
            let output = compile_command.output.as_str();
            dir_config(&config::OUTPUT_DIR, output.to_string());
            dir_config(&config::ROOT_DIR, compile_command.root.to_string());

            let base_url = compile_command.base.to_string();
            let base_url = match base_url.ends_with("/") {
                true => base_url,
                false => format!("{}/", base_url),
            };
            dir_config(&config::BASE_URL, base_url);

            match compile_to_html(input) {
                Err(err) => eprintln!("{:?}", err),
                _ => (),
            }
            kodama::compile_links();
        }
        Command::Clean(clean_command) => {
            dir_config(&config::ROOT_DIR, clean_command.root.to_string());
            let _ = config::delete_all_build_files();
        }
    }
}
