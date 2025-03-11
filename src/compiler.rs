pub mod callback;
pub mod counter;
pub mod parser;
pub mod section;
pub mod state;
pub mod taxon;
pub mod typst;
pub mod writer;

use std::{collections::HashMap, fmt::Debug, path::Path};

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
    let workspace = all_source_files(Path::new(workspace_dir)).unwrap();

    for (slug, ext) in &workspace.slug_exts {
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
            let shallow = match ext.as_str() {
                "md" => parse_markdown(slug)?,
                "typst" => parse_typst(slug, workspace_dir)?,
                _ => panic!(),
            };
            let serialized = serde_json::to_string(&shallow).unwrap();
            std::fs::write(entry_path_buf, serialized).map_err(|e| {
                CompileError::IO(Some(concat!(file!(), '#', line!())), e, entry_path_str)
            })?;

            shallow
        };

        state.residued.insert(slug.to_string(), shallow);
    }

    state.compile_all();

    Writer::write_needed_slugs(
        &workspace.slug_exts.into_iter().map(|x| x.0).collect(),
        &state,
    );

    Ok(())
}

pub fn should_ignored_file(path: &Path) -> bool {
    let name = path.file_name().unwrap();
    name == "README.md"
}

pub fn should_ignored_dir(path: &Path) -> bool {
    let name = path.file_name().unwrap();
    name.to_str()
        .map_or(false, |s| s.starts_with('.') || s.starts_with('_'))
}

pub fn is_source(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| matches!(s, "md" | "typst"))
        .unwrap_or(false)
}

fn err_collide(path: &String, ext: &String) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        format!("{} collides with another file with '.{}'", path, ext),
    )
}

/**
 * collect all source file paths in workspace dir
 */
pub fn all_source_files(root_dir: &Path) -> Result<Workspace, std::io::Error> {
    let root_dir_str = root_dir.to_str().unwrap();
    let offset = root_dir_str.len();
    let mut slug_exts = HashMap::new();
    let to_slug_ext = |s: String| slug::to_slug_ext(&s[offset..]);

    for entry in std::fs::read_dir(root_dir)? {
        let path = entry?.path();
        if path.is_file() && is_source(&path) && !should_ignored_file(&path) {
            let path = posix_style(path.to_str().unwrap());
            let (slug, ext) = to_slug_ext(path.to_string());
            if let Some(ext) = slug_exts.insert(slug, ext) {
                return Err(err_collide(&path, &ext));
            };
        } else if path.is_dir() && !should_ignored_dir(&path) {
            files_match_with(
                &path,
                &is_source,
                &should_ignored_dir,
                &mut slug_exts,
                &to_slug_ext,
                &err_collide,
            )?;
        }
    }

    Ok(Workspace { slug_exts })
}

#[derive(Debug)]
pub struct Workspace {
    pub slug_exts: HashMap<String, String>,
}
