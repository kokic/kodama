// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Alias Qli (@AliasQli), Spore (@s-cerevisiae)

mod anonymous_slug;
mod artifacts;
pub mod callback;
pub mod counter;
pub mod html_parser;
mod incremental;
pub mod parser;
mod rss;
pub mod section;
mod serve_session;
mod source_scan;
mod stale;
pub mod state;
mod subtree_slug;
pub mod taxon;
pub mod typst;
pub mod writer;

use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::BufReader,
};

use camino::{Utf8Path, Utf8PathBuf};
use eyre::{eyre, WrapErr};
use parser::parse_markdown_sections;
use section::{HTMLContent, UnresolvedSection};
use serde::{Deserialize, Serialize};
use typst::parse_typst_sections;
use writer::Writer;

use crate::{
    entry::{MetaData, KEY_INTERNAL_ANON_SUBTREE},
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
pub use serve_session::ServeCompileSession;
pub use source_scan::{
    all_trees_source, all_trees_source_readonly, sync_typst_svg_assets, Workspace,
};

pub type DirtySet = HashSet<Utf8PathBuf>;
pub type UnresolvedSections = HashMap<Slug, UnresolvedSection>;
pub type SourceSectionsIndex = HashMap<Slug, Vec<Slug>>;
pub type ParsedSections = Vec<(Slug, UnresolvedSection)>;

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

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct CachedSourceEntry {
    pub sections: Vec<CachedSection>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct CachedSection {
    pub slug: Slug,
    pub section: UnresolvedSection,
}

pub fn compile(
    workspace: Workspace,
    dirty_paths: Option<&DirtySet>,
    outputs: CompileOutputs,
) -> eyre::Result<()> {
    let stale_slugs = cleanup_stale_slug_artifacts(&workspace)
        .wrap_err("failed to clean stale slug artifacts")?;
    let shallows = collect_shallows(&workspace, dirty_paths)?;
    compile_from_shallows(&workspace, &shallows, dirty_paths, outputs, stale_slugs)
}

pub fn refresh_indexes(
    workspace: &Workspace,
    dirty_paths: Option<&DirtySet>,
) -> eyre::Result<HashMap<Slug, OrderedMap<String, HTMLContent>>> {
    let shallows = collect_shallows(workspace, dirty_paths)?;
    Ok(indexes_from_shallows(&shallows))
}

pub(super) fn compile_from_shallows(
    workspace: &Workspace,
    shallows: &UnresolvedSections,
    dirty_paths: Option<&DirtySet>,
    outputs: CompileOutputs,
    stale_slugs: HashSet<Slug>,
) -> eyre::Result<()> {
    let mut all_slugs: Vec<Slug> = shallows
        .iter()
        .filter_map(|(slug, section)| (!is_internal_anonymous_subtree(section)).then_some(*slug))
        .collect();
    all_slugs.sort();

    let indexes = outputs.indexes.then(|| indexes_from_shallows(shallows));

    let state = state::compile_all(shallows)?;
    let slugs_to_write: Vec<Slug> = match dirty_paths {
        Some(dirty_paths) => {
            let dirty_slugs = dirty_source_slugs(workspace, dirty_paths);
            if !stale_slugs.is_empty() {
                all_slugs.clone()
            } else if dirty_slugs.is_empty() {
                Vec::new()
            } else {
                affected_slugs_from_dirty(&state, &dirty_slugs)
                    .into_iter()
                    .filter(|slug| {
                        shallows
                            .get(slug)
                            .is_some_and(|section| !is_internal_anonymous_subtree(section))
                    })
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
    sync_optional_output(
        indexes_path.as_path(),
        indexes_payload.as_deref(),
        "indexes",
    )?;

    if environment::is_publish() {
        let feed_path = environment::feed_path(output_dir.as_path());
        let feed_payload = if environment::publish_rss() {
            rss::ensure_publish_rss_base_url_is_absolute()?;
            Some(rss::feed_xml(&state)?)
        } else {
            None
        };
        sync_optional_output(feed_path.as_path(), feed_payload.as_deref(), "rss feed")?;
    }

    Ok(())
}

fn indexes_from_shallows(
    shallows: &UnresolvedSections,
) -> HashMap<Slug, OrderedMap<String, HTMLContent>> {
    shallows
        .iter()
        .filter(|(_, section)| !is_internal_anonymous_subtree(section))
        .map(|(slug, section)| (*slug, section.metadata.0.clone()))
        .collect()
}

fn is_internal_anonymous_subtree(section: &UnresolvedSection) -> bool {
    section
        .metadata
        .get_str(KEY_INTERNAL_ANON_SUBTREE)
        .is_some_and(|value| value == "true")
}

pub(super) fn collect_shallows(
    workspace: &Workspace,
    dirty_paths: Option<&DirtySet>,
) -> eyre::Result<UnresolvedSections> {
    Ok(collect_shallows_with_sources(workspace, dirty_paths)?.0)
}

pub(super) fn collect_shallows_with_sources(
    workspace: &Workspace,
    dirty_paths: Option<&DirtySet>,
) -> eyre::Result<(UnresolvedSections, SourceSectionsIndex)> {
    let mut shallows = HashMap::new();
    let mut source_sections = HashMap::new();

    for (&source_slug, &ext) in &workspace.slug_exts {
        let sections = load_shallow_sections(source_slug, ext, dirty_paths)?;
        let produced_slugs: Vec<Slug> = sections.iter().map(|(slug, _)| *slug).collect();

        for (slug, shallow) in sections {
            if shallows.insert(slug, shallow).is_some() {
                return Err(eyre!(
                    "section slug collision: `{}` is generated multiple times (latest from `{}`)",
                    slug,
                    source_slug
                ));
            }
        }

        source_sections.insert(source_slug, produced_slugs);
    }

    Ok((shallows, source_sections))
}

pub(crate) fn parse_source_sections(source_slug: Slug, ext: Ext) -> eyre::Result<ParsedSections> {
    let mut sections = match ext {
        Ext::Markdown => parse_markdown_sections(source_slug)
            .wrap_err_with(|| eyre!("failed to parse markdown file `{source_slug}.{ext}`"))?,
        Ext::Typst => parse_typst_sections(source_slug, environment::typst_root_dir())
            .wrap_err_with(|| eyre!("failed to parse typst file `{source_slug}.{ext}`"))?,
    };

    for (_, section) in &mut sections {
        section.metadata.compute_textual_attrs();
    }
    Ok(sections)
}

pub(super) fn write_entry_cache(
    entry_path: &Utf8Path,
    sections: &[(Slug, UnresolvedSection)],
) -> eyre::Result<()> {
    let serialized = serde_json::to_string(&CachedSourceEntry {
        sections: sections
            .iter()
            .map(|(slug, section)| CachedSection {
                slug: *slug,
                section: section.clone(),
            })
            .collect(),
    })
    .wrap_err_with(|| eyre!("failed to serialize entry for `{}`", entry_path))?;
    std::fs::write(entry_path, serialized)
        .wrap_err_with(|| eyre!("failed to write entry to `{}`", entry_path))?;
    Ok(())
}

fn read_entry_cache(entry_path: &Utf8Path, source_slug: Slug) -> eyre::Result<ParsedSections> {
    let entry_file = BufReader::new(
        File::open(entry_path)
            .wrap_err_with(|| eyre!("failed to open entry file at `{}`", entry_path))?,
    );

    if let Ok(cached) = serde_json::from_reader::<_, CachedSourceEntry>(entry_file) {
        return Ok(cached
            .sections
            .into_iter()
            .map(|cached| (cached.slug, cached.section))
            .collect());
    }

    // Backward compatibility: older versions cached a single section value.
    let entry_file = BufReader::new(
        File::open(entry_path)
            .wrap_err_with(|| eyre!("failed to reopen entry file at `{}`", entry_path))?,
    );
    let section: UnresolvedSection = serde_json::from_reader(entry_file)
        .wrap_err_with(|| eyre!("failed to deserialize entry file at `{}`", entry_path))?;
    Ok(vec![(source_slug, section)])
}

fn load_shallow_sections(
    source_slug: Slug,
    ext: Ext,
    dirty_paths: Option<&DirtySet>,
) -> eyre::Result<ParsedSections> {
    let relative_path = source_relative_path(source_slug, ext);
    let is_modified = is_source_modified(relative_path.as_path(), dirty_paths)
        .wrap_err_with(|| eyre!("failed to verify hash of `{relative_path}`"))?;
    let entry_path = environment::entry_file_path(&relative_path);

    if !is_modified && entry_path.exists() {
        let mut sections = read_entry_cache(entry_path.as_path(), source_slug)?;
        for (_, section) in &mut sections {
            section.metadata.compute_textual_attrs();
        }
        return Ok(sections);
    }

    let sections = parse_source_sections(source_slug, ext)?;
    write_entry_cache(entry_path.as_path(), &sections)?;
    Ok(sections)
}
