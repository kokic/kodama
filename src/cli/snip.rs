use std::collections::HashMap;

use eyre::{bail, eyre, WrapErr};
use serde::Serialize;

use crate::{
    compiler::section::HTMLContent, config, entry, environment, ordered_map::OrderedMap, slug::Slug,
};

#[derive(clap::Args)]
pub struct SnipCommand {
    /// Path to the configuration file (e.g., "Kodama.toml").
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG_PATH.into())]
    config: String,
}

#[derive(Serialize)]
struct Snippet {
    prefix: String,
    body: [String; 1],
    // description: String,
}

/// This function invoked the [`environment::init_environment`] function to initialize the environment]
pub fn snip(command: &SnipCommand) -> eyre::Result<()> {
    let config_path = &command.config;
    environment::init_environment(config_path.into(), environment::BuildMode::Serve)?;

    let output_dir = environment::root_dir().join(environment::serve_dir());
    let indexes_path = environment::indexes_path(&output_dir);

    // Check if the indexes file exists
    if !indexes_path.exists() {
        bail!("Indexes file not found. Please run `kodama serve` first.");
    }

    let indexes_content = std::fs::read_to_string(&indexes_path)
        .wrap_err_with(|| eyre!("Failed to read indexes file at `{}`", indexes_path))?;
    let indexes: HashMap<Slug, OrderedMap<String, HTMLContent>> =
        serde_json::from_str(&indexes_content)
            .wrap_err_with(|| eyre!("Failed to parse indexes JSON from `{}`", indexes_path))?;

    let snippets: HashMap<&str, Snippet> = indexes
        .iter()
        .filter_map(|(slug, metadata)| {
            let prefix = metadata.get(entry::KEY_TITLE)?.as_str()?;            
            let slug_str = slug.as_str();

            let ext = metadata
                .get(entry::KEY_EXT)?                
                .as_str()?;

            let trees_dir = environment::trees_dir_without_root();
            let url = format!("/{}/{}.{}", trees_dir, slug_str, ext);

            let label = prefix.to_lowercase().replace(" ", "-");
            let body = [format!("[{label}]: {url}")];

            Some((
                slug_str,
                Snippet {
                    prefix: prefix.to_string(),
                    body,
                },
            ))
        })
        .collect();

    let snippets_path = environment::root_dir()
        .join(".vscode")
        .join("markdown.code-snippets");
    environment::create_parent_dirs(&snippets_path);

    let serialized = serde_json::to_string_pretty(&snippets)
        .wrap_err_with(|| eyre!("failed to serialize snippets to JSON"))?;
    std::fs::write(&snippets_path, serialized)
        .wrap_err_with(|| eyre!("failed to write snippets to `{}`", snippets_path))?;

    Ok(())
}
