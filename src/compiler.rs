pub mod callback;
pub mod counter;
pub mod parser;
pub mod section;
pub mod state;
pub mod taxon;
pub mod typst;
pub mod writer;

use std::{ffi::OsStr, fmt::Debug, path::Path};

use parser::parse_markdown;
use section::{HTMLContent, ShallowSection};
use state::CompileState;
use typst::parse_typst;
use writer::Writer;

use crate::{
    config::{self, files_match_with, verify_and_file_hash},
    slug::{self, posix_style},
};

#[allow(dead_code)]
#[derive(Debug)]
pub enum CompileError {
    IO(Option<&'static str>, std::io::Error, String),
    Syntax(Option<&'static str>, Box<dyn Debug>, String),
}

pub fn compile_all(workspace_dir: &str) -> Result<(), CompileError> {
    let mut state = CompileState::new();

    fn compile_filetype<F: Fn() -> Result<ShallowSection, CompileError>>(
        slug: &str,
        ext: &str,
        parse: F,
    ) -> Result<ShallowSection, CompileError> {
        let relative_path = format!("{}.{}", slug, ext);

        let is_modified = verify_and_file_hash(&relative_path).map_err(|e| {
            CompileError::IO(
                Some(concat!(file!(), '#', line!())),
                e,
                relative_path.to_string(),
            )
        })?;

        let entry_path_str = format!("{}.entry", relative_path);
        let entry_path_buf = config::entry_path(&entry_path_str);

        let shallow = if !is_modified && entry_path_buf.exists() {
            let serialized = std::fs::read_to_string(entry_path_buf).map_err(|e| {
                let position = Some(concat!(file!(), '#', line!()));
                CompileError::IO(position, e, entry_path_str)
            })?;

            let shallow: ShallowSection = serde_json::from_str(&serialized).unwrap();
            shallow
        } else {
            let shallow = parse()?;
            let serialized = serde_json::to_string(&shallow).unwrap();
            std::fs::write(entry_path_buf, serialized).map_err(|e| {
                CompileError::IO(Some(concat!(file!(), '#', line!())), e, entry_path_str)
            })?;

            shallow
        };

        Ok(shallow)
    }

    let workspace = all_files(Path::new(workspace_dir), is_markdown).unwrap();
    for slug in &workspace.slugs {
        let shallow = compile_filetype(slug, "md", || parse_markdown(slug))?;
        state.residued.insert(slug.to_string(), shallow);
    }

    let workspace = all_files(Path::new(workspace_dir), is_typst).unwrap();
    for slug in &workspace.slugs {
        let shallow = compile_filetype(slug, "typst", || parse_typst(slug, workspace_dir))?;
        state.residued.insert(slug.to_string(), shallow);
    }

    state.compile_all();

    Writer::write_needed_slugs(&workspace.slugs, &state);

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

pub fn is_typst(path: &Path) -> bool {
    path.extension() == Some(OsStr::new("typst"))
}

/**
 * collect all source file paths in workspace dir
 */
pub fn all_files<F: Fn(&Path) -> bool>(
    root_dir: &Path,
    predicate: F,
) -> Result<Workspace, Box<std::io::Error>> {
    let root_dir = root_dir.to_str().unwrap();
    let offset = root_dir.len();
    let mut slugs: Vec<String> = vec![];
    let to_slug = |s: String| slug::to_slug(&s[offset..]);

    for entry in std::fs::read_dir(root_dir)? {
        let path = entry?.path();
        if path.is_file() && predicate(&path) && !should_ignored_file(&path) {
            let path = posix_style(path.to_str().unwrap());
            slugs.push(to_slug(path));
        } else if path.is_dir() && !should_ignored_dir(&path) {
            files_match_with(&path, &predicate, &mut slugs, &to_slug)?;
        }
    }

    Ok(Workspace { slugs })
}

#[derive(Debug)]
pub struct Workspace {
    pub slugs: Vec<String>,
}
