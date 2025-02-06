pub mod counter;
pub mod parser;
pub mod section;
pub mod taxon;
pub mod state;
pub mod writer;

use std::{ffi::OsStr, path::Path};

use parser::parse_markdown;
use section::{HTMLContent, ShallowSection};
use state::CompileState;
use writer::Writer;

use crate::{
    config::{self, files_match_with, verify_and_update_file_hash},
    slug::{self, posix_style},
};

#[allow(dead_code)]
#[derive(Debug)]
pub enum CompileError {
    IO(Option<String>, std::io::Error, String),
}

pub fn compile_all(workspace_dir: &str) -> Result<(), CompileError> {
    let workspace = all_markdown_file(Path::new(workspace_dir)).unwrap();
    let mut state = CompileState::new();
    let mut write_slugs: Vec<String> = vec![];

    for slug in workspace.slugs {
        let relative_path = format!("{}.md", slug);

        let is_modified = verify_and_update_file_hash(&relative_path).map_err(|e| {
            let position = Some("compiler::compile_all@is_modified".to_owned());
            CompileError::IO(position, e, relative_path.to_string())
        })?;

        let entry_path_str = format!("{}.entry", relative_path);
        let entry_path_buf = config::entry_path(&entry_path_str);

        let shallow = if !is_modified && entry_path_buf.exists() {
            let serialized = std::fs::read_to_string(entry_path_buf).map_err(|e| {
                let position = Some("compiler::compile_all@serialized:read".to_owned());
                CompileError::IO(position, e, entry_path_str)
            })?;

            let shallow: ShallowSection = serde_json::from_str(&serialized).unwrap();
            shallow
        } else {
            let shallow = parse_markdown(&slug)?;
            let serialized = serde_json::to_string(&shallow).unwrap();
            std::fs::write(entry_path_buf, serialized).map_err(|e| {
                let position = Some("compiler::compile_all@serialized:write".to_owned());
                CompileError::IO(position, e, entry_path_str)
            })?;

            write_slugs.push(slug.to_string());
            shallow
        };

        state.residued.insert(slug, shallow);
    }

    state.compile_all();

    write_slugs
        .iter()
        .for_each(|slug| match state.compiled.get(slug) {
            /*
             * No need for `state.compiled.remove(slug)` here,
             * because writing to a file does not require a mutable reference
             * of the [`Section`].
             */
            None => eprintln!("Slug `{}` not in compiled entries.", slug),
            Some(section) => Writer::write(section, &state),
        });

    Ok(())
}

pub fn should_ignored_file(path: &Path) -> bool {
    let name = path.file_name().unwrap();
    name == "README.md"
}

pub fn should_ignored_dir(path: &Path) -> bool {
    let name = path.file_name().unwrap();
    name == config::CACHE_DIR_NAME
}

pub fn is_markdown(path: &Path) -> bool {
    path.extension() == Some(OsStr::new("md"))
}

/**
 * collect all markdown source file paths in workspace dir
 */
pub fn all_markdown_file(root_dir: &Path) -> Result<Workspace, Box<std::io::Error>> {
    let root_dir = root_dir.to_str().unwrap();
    let offset = root_dir.len();
    let mut slugs: Vec<String> = vec![];
    let to_slug = |s: String| slug::to_slug(&s[offset..]);

    for entry in std::fs::read_dir(root_dir)? {
        let path = entry?.path();
        if path.is_file() && is_markdown(&path) && !should_ignored_file(&path) {
            let path = posix_style(path.to_str().unwrap());
            slugs.push(to_slug(path));
        } else if path.is_dir() && !should_ignored_dir(&path) {
            files_match_with(&path, &is_markdown, &mut slugs, &to_slug)?;
        }
    }

    Ok(Workspace { slugs })
}

#[derive(Debug)]
pub struct Workspace {
    pub slugs: Vec<String>,
}
