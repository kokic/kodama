use std::path::PathBuf;

pub const DEFAULT_TEMPLATE: &str = "./template";

#[derive(clap::Args)]
pub struct NewCommand {
    /// Path to section to create.
    #[arg(required = true)]
    pub path: PathBuf,

    /// Path to the template file to use for the new section.
    #[arg(short, long, default_value_t = DEFAULT_TEMPLATE.to_string())]
    pub template: String,
}

pub fn new(command: &NewCommand) -> eyre::Result<()> {
    let template = &command.template;
    let default_not_exists = template == DEFAULT_TEMPLATE && !std::fs::exists(&template)?;

    let template = if default_not_exists {
        String::new()
    } else {
        std::fs::read_to_string(&template)
            .map_err(|e| eyre::eyre!("Failed to read template file: {}", e))?
    };

    std::fs::write(&command.path, template)
        .map_err(|e| eyre::eyre!("Failed to create section file: {}", e))?;

    println!("Created new section at: {}", command.path.display());
    Ok(())
}
