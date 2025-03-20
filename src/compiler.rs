pub mod callback;
pub mod counter;
pub mod html_parser;
pub mod parser;
pub mod section;
pub mod state;
pub mod taxon;
pub mod typst;
pub mod writer;

use std::{collections::HashMap, fs::File, io::BufReader, path::Path};

use eyre::{bail, eyre, WrapErr};
use parser::parse_markdown;
use section::{HTMLContent, ShallowSection};
use typst::parse_typst;
use walkdir::WalkDir;
use writer::Writer;

use crate::{
    config::{self, verify_and_file_hash},
    slug::{self, Ext, Slug},
};

pub fn compile_all(workspace_dir: &str) -> eyre::Result<()> {
    let workspace = all_source_files(Path::new(workspace_dir))?;
    let mut shallows = HashMap::new();

    for (&slug, &ext) in &workspace.slug_exts {
        let relative_path = format!("{}.{}", slug, ext);

        let is_modified = verify_and_file_hash(&relative_path)
            .wrap_err_with(|| eyre!("failed to verify hash of `{relative_path}`"))?;

        let entry_path_str = format!("{}.entry", relative_path);
        let entry_path_buf = config::entry_path(&entry_path_str);

        let shallow = if !is_modified && entry_path_buf.exists() {
            let entry_file = BufReader::new(File::open(&entry_path_buf).wrap_err_with(|| {
                eyre!(
                    "failed to open entry file at `{}`",
                    entry_path_buf.display()
                )
            })?);
            let shallow: ShallowSection =
                serde_json::from_reader(entry_file).wrap_err_with(|| {
                    eyre!(
                        "failed to deserialize entry file at `{}`",
                        entry_path_buf.display()
                    )
                })?;
            shallow
        } else {
            let shallow = match ext {
                Ext::Markdown => parse_markdown(slug)
                    .wrap_err_with(|| eyre!("failed to parse markdown file `{slug}.{ext}`"))?,
                Ext::Typst => parse_typst(slug, workspace_dir)
                    .wrap_err_with(|| eyre!("failed to parse typst file `{slug}.{ext}`"))?,
            };
            let serialized = serde_json::to_string(&shallow).unwrap();
            std::fs::write(&entry_path_buf, serialized).wrap_err_with(|| {
                eyre!("failed to write entry to `{}`", entry_path_buf.display())
            })?;

            shallow
        };

        shallows.insert(slug, shallow);
    }

    let state = state::compile_all(shallows)?;

    Writer::write_needed_slugs(workspace.slug_exts.into_iter().map(|x| x.0), &state);

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

/**
 * collect all source file paths in workspace dir
 */
pub fn all_source_files(root_dir: &Path) -> eyre::Result<Workspace> {
    let mut slug_exts = HashMap::new();
    let to_slug_ext = |p: &Path| {
        let p = p.strip_prefix(root_dir).unwrap_or(p);
        let ext = p.extension()?.to_str()?.parse().ok()?;
        let slug = Slug::new(slug::pretty_path(&p.with_extension("")));
        Some((slug, ext))
    };

    let failed_to_read_dir = |dir: &Path| eyre!("failed to read directory `{}`", dir.display());
    let file_collide = |p: &Path, e: Ext| {
        eyre!(
            "`{}` collides with `{}`",
            p.display(),
            p.with_extension(e.to_string()).display(),
        )
    };
    for entry in std::fs::read_dir(root_dir).wrap_err_with(|| failed_to_read_dir(root_dir))? {
        let path = entry.wrap_err_with(|| failed_to_read_dir(root_dir))?.path();
        if path.is_file() && !should_ignored_file(&path) {
            let Some((slug, ext)) = to_slug_ext(&path) else {
                continue;
            };
            if let Some(ext) = slug_exts.insert(slug, ext) {
                bail!(file_collide(&path, ext));
            };
        } else if path.is_dir() && !should_ignored_dir(&path) {
            for entry in WalkDir::new(&path)
                .follow_links(true)
                .into_iter()
                .filter_entry(|e| {
                    let path = e.path();
                    path.is_file() || !should_ignored_dir(path)
                })
            {
                let path = entry
                    .wrap_err_with(|| failed_to_read_dir(&path))?
                    .into_path();
                if path.is_file() {
                    let Some((slug, ext)) = to_slug_ext(&path) else {
                        continue;
                    };
                    if let Some(ext) = slug_exts.insert(slug, ext) {
                        bail!(file_collide(&path, ext));
                    }
                }
            }
        }
    }

    Ok(Workspace { slug_exts })
}

#[derive(Debug)]
pub struct Workspace {
    pub slug_exts: HashMap<Slug, Ext>,
}
