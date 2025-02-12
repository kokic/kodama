mod compiler;
mod config;
mod entry;
mod html_flake;
mod html_macro;
mod process;
mod recorder;
mod slug;
mod typst_cli;

use clap::Parser;
use config::{CompileConfig, FooterMode};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Compile current workspace dir to HTMLs.
    #[command(visible_alias = "c")]
    Compile(CompileCommand),

    /// Clean build files (.cache & publish).
    Clean(CleanCommand),
}

#[derive(clap::Args)]
struct CompileCommand {
    /// Base URL or publish URL (e.g. https://www.example.com/)
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG.base_url.into())]
    base: String,

    /// Path to output dir.
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG.output_dir.into())]
    output: String,

    /// Configures the project root (for absolute paths)
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG.root_dir.into())]
    root: String,

    /// Disable pretty urls (`/page` to `/page.html`)
    #[arg(short, long, default_value_t = false)]
    disable_pretty_urls: bool,

    /// Hide parents part in slug (e.g. `tutorials/install` to `install`)
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG.short_slug)]
    short_slug: bool,

    /// Specify the inline mode for the footer sections
    #[arg(short, long, default_value_t = FooterMode::Link)]
    footer_mode: FooterMode, 
}

#[derive(clap::Args)]
struct CleanCommand {
    /// Path to output dir.
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG.output_dir.into())]
    output: String,

    /// Configures the project root (for absolute paths)
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG.root_dir.into())]
    root: String,

    /// Clean markdown hash files.
    #[arg(short, long)]
    markdown: bool,

    /// Clean typst hash files.
    #[arg(long)]
    typst: bool,

    /// Clean html hash files.
    #[arg(long)]
    html: bool,
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Command::Compile(compile_command) => {
            let root = &compile_command.root;
            let output = &compile_command.output;

            config::mutex_set(&config::CONFIG, CompileConfig::new(
                root.to_string(), 
                output.to_string(), 
                compile_command.base.to_string(), 
                compile_command.disable_pretty_urls,
                compile_command.short_slug, 
                compile_command.footer_mode.clone(), 
            ));

            match compiler::compile_all(root) {
                Err(err) => eprintln!("{:?}", err),
                Ok(_) => (),
            }
        }
        Command::Clean(clean_command) => {
            config::mutex_set(&config::CONFIG, CompileConfig::new(
                clean_command.root.to_string(), 
                clean_command.output.to_string(), 
                config::DEFAULT_CONFIG.base_url.into(), 
                false,
                config::DEFAULT_CONFIG.short_slug, 
                FooterMode::Link, 
            ));

            let cache_dir = &config::get_cache_dir();

            clean_command.markdown.then(|| {
                let _ = config::delete_all_with(&cache_dir, &|s| {
                    s.to_str().unwrap().ends_with(".md.hash")
                });
            });
            
            clean_command.typst.then(|| {
                let _ = config::delete_all_with(&cache_dir, &|s| {
                    s.to_str().unwrap().ends_with(".typ.hash")
                });
            });

            clean_command.html.then(|| {
                let _ = config::delete_all_with(&cache_dir, &|s| {
                    s.to_str().unwrap().ends_with(".html.hash")
                });
            });
        }
    }
}
