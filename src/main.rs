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

    /// Clean all build files (.cache & publish).
    Clean(CleanCommand),
}

#[derive(clap::Args)]
struct CompileCommand {
    /// Base URL or publish URL (e.g. https://www.example.com/)
    #[arg(short, long, default_value_t = format!("/"))]
    base: String,

    /// Path to output dir.
    #[arg(short, long, default_value_t = format!("./publish"))]
    output: String,

    /// Configures the project root (for absolute paths)
    #[arg(short, long, default_value_t = format!("./"))]
    root: String,

    /// Disable pretty urls (`/page` to `/page.html`)
    #[arg(short, long)]
    disable_pretty_urls: bool,

    /// Hide parents part in slug (e.g. `tutorials/install` to `install`)
    #[arg(short, long)]
    short_slug: bool,
}

#[derive(clap::Args)]
struct CleanCommand {
    /// Path to output dir.
    #[arg(short, long, default_value_t = format!("./publish"))]
    output: String,

    /// Configures the project root (for absolute paths)
    #[arg(short, long, default_value_t = format!("./"))]
    root: String,

    /// Clean markdown hash files.
    #[arg(short, long)]
    markdown: bool,

    /// Clean typst hash files.
    #[arg(short, long)]
    typst: bool,
}

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Command::Compile(compile_command) => {
            let root = &compile_command.root;
            let output = &compile_command.output;
            config::mutex_set(&config::OUTPUT_DIR, output.to_string());
            config::mutex_set(&config::ROOT_DIR, root.to_string());
            if compile_command.disable_pretty_urls {
                config::mutex_set(&config::PAGE_SUFFIX, ".html".to_string());
            }
            config::mutex_set(&config::SHORT_SLUG, compile_command.short_slug);

            config::set_base_url(compile_command.base.to_string());

            match compiler::compile_all(root) {
                Err(err) => eprintln!("{:?}", err),
                Ok(_) => (),
            }
        }
        Command::Clean(clean_command) => {
            let output = clean_command.output.as_str();
            config::mutex_set(&config::OUTPUT_DIR, output.to_string());
            config::mutex_set(&config::ROOT_DIR, clean_command.root.to_string());

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
        }
    }
}
