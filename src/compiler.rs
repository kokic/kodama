// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Alias Qli (@AliasQli), Spore (@s-cerevisiae)

pub mod callback;
pub mod counter;
pub mod html_parser;
pub mod parser;
pub mod section;
pub mod state;
pub mod taxon;
pub mod typst;
pub mod writer;
mod artifacts;
mod incremental;
mod source_scan;
mod stale;

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::BufReader,
};

use camino::Utf8PathBuf;
use eyre::{eyre, WrapErr};
use parser::parse_markdown;
use section::{HTMLContent, UnresolvedSection};
use typst::parse_typst;
use writer::Writer;

use crate::{
    environment,
    ordered_map::OrderedMap,
    slug::{Ext, Slug},
};

use self::{
    artifacts::{graph_snapshot, sync_optional_output},
    incremental::{
        affected_slugs_from_dirty, dirty_source_slugs, is_source_modified, source_relative_path,
    },
    stale::cleanup_stale_slug_artifacts,
};

pub use incremental::expand_dirty_paths;
pub use source_scan::{all_trees_source, Workspace};

pub type DirtySet = HashSet<Utf8PathBuf>;

#[derive(Debug, Clone, Copy)]
pub struct CompileOutputs {
    pub indexes: bool,
    pub graph: bool,
}

impl Default for CompileOutputs {
    fn default() -> Self {
        Self {
            indexes: true,
            graph: true,
        }
    }
}

pub fn compile(
    workspace: Workspace,
    dirty_paths: Option<&DirtySet>,
    outputs: CompileOutputs,
) -> eyre::Result<()> {
    let mut shallows = HashMap::new();
    let all_slugs: Vec<Slug> = workspace.slug_exts.keys().copied().collect();
    let stale_slugs = cleanup_stale_slug_artifacts(&workspace)
        .wrap_err("failed to clean stale slug artifacts")?;

    for (&slug, &ext) in &workspace.slug_exts {
        let relative_path = source_relative_path(slug, ext);

        let is_modified = is_source_modified(relative_path.as_path(), dirty_paths)
            .wrap_err_with(|| eyre!("failed to verify hash of `{relative_path}`"))?;
        let entry_path = environment::entry_file_path(&relative_path);
        let shallow = if !is_modified && entry_path.exists() {
            let entry_file = BufReader::new(
                File::open(&entry_path)
                    .wrap_err_with(|| eyre!("failed to open entry file at `{}`", entry_path))?,
            );
            let shallow: UnresolvedSection = serde_json::from_reader(entry_file)
                .wrap_err_with(|| eyre!("failed to deserialize entry file at `{}`", entry_path))?;
            shallow
        } else {
            let shallow = match ext {
                Ext::Markdown => parse_markdown(slug)
                    .wrap_err_with(|| eyre!("failed to parse markdown file `{slug}.{ext}`"))?,
                Ext::Typst => parse_typst(slug, environment::typst_root_dir())
                    .wrap_err_with(|| eyre!("failed to parse typst file `{slug}.{ext}`"))?,
            };
            let serialized = serde_json::to_string(&shallow)
                .wrap_err_with(|| eyre!("failed to serialize entry for `{slug}.{ext}`"))?;
            std::fs::write(&entry_path, serialized)
                .wrap_err_with(|| eyre!("failed to write entry to `{}`", entry_path))?;

            shallow
        };

        shallows.insert(slug, shallow);
    }

    let indexes = outputs.indexes.then(|| {
        shallows
            .iter()
            .map(|(slug, section)| (*slug, section.metadata.0.clone()))
            .collect::<HashMap<Slug, OrderedMap<String, HTMLContent>>>()
    });

    let state = state::compile_all(shallows)?;
    let slugs_to_write: Vec<Slug> = match dirty_paths {
        Some(dirty_paths) => {
            let dirty_slugs = dirty_source_slugs(&workspace, dirty_paths);
            if dirty_slugs.is_empty() || !stale_slugs.is_empty() {
                all_slugs.clone()
            } else {
                affected_slugs_from_dirty(&state, &dirty_slugs)
                    .into_iter()
                    .collect()
            }
        }
        None => all_slugs.clone(),
    };

    Writer::write_needed_slugs(slugs_to_write, &state)
        .wrap_err("failed to write compiled HTML files")?;

    let graph_payload = if outputs.graph {
        let graph = graph_snapshot(&state);
        Some(
            serde_json::to_string(&graph)
                .wrap_err_with(|| eyre!("failed to serialize graph to JSON"))?,
        )
    } else {
        None
    };
    let indexes_payload = if let Some(indexes) = indexes {
        Some(
            serde_json::to_string(&indexes)
                .wrap_err_with(|| eyre!("failed to serialize indexes to JSON"))?,
        )
    } else {
        None
    };

    let output_dir = environment::output_dir();
    let graph_path = environment::graph_path(output_dir.as_path());
    sync_optional_output(graph_path.as_path(), graph_payload.as_deref(), "graph")?;

    let indexes_path = environment::indexes_path(output_dir.as_path());
    sync_optional_output(indexes_path.as_path(), indexes_payload.as_deref(), "indexes")?;

    Ok(())
}

