// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Alias Qli (@AliasQli), Spore (@s-cerevisiae)

use std::{
    collections::BTreeMap,
    io::Write,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use camino::Utf8Path;
use eyre::{eyre, WrapErr};
use serde::Serialize;

use crate::{
    entry::{MetaData, KEY_INTERNAL_ANON_SUBTREE},
    environment,
    slug::Slug,
};

use super::{stale::remove_file_if_exists, state};

static ARTIFACT_ATOMIC_WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Serialize)]
pub(super) struct GraphSnapshot {
    sections: BTreeMap<Slug, GraphSection>,
}

#[derive(Debug, Serialize)]
struct GraphSection {
    parent: Slug,
    parent_specified: bool,
    references: Vec<Slug>,
    backlinks: Vec<Slug>,
}

pub(super) fn graph_snapshot(state: &state::CompileState) -> GraphSnapshot {
    let mut sections = BTreeMap::new();
    let mut slugs: Vec<Slug> = state.compiled().keys().copied().collect();
    slugs.sort();

    let is_internal = |slug: Slug| {
        state
            .compiled()
            .get(&slug)
            .and_then(|section| section.metadata.get_str(KEY_INTERNAL_ANON_SUBTREE))
            .is_some_and(|value| value == "true")
    };

    for slug in slugs {
        let section = state
            .compiled()
            .get(&slug)
            .expect("slug collected from compiled map must exist");
        if is_internal(slug) {
            continue;
        }
        let callback = state.callback().0.get(&slug);
        let mut parent = callback.map_or(Slug::new("index"), |value| value.parent);
        while is_internal(parent) {
            parent = state
                .callback()
                .0
                .get(&parent)
                .map_or(Slug::new("index"), |value| value.parent);
        }
        let parent_specified = callback.is_some_and(|value| value.is_parent_specified);

        let mut references: Vec<Slug> = section
            .references
            .iter()
            .copied()
            .filter(|reference| !is_internal(*reference))
            .collect();
        references.sort();

        let mut backlinks: Vec<Slug> = callback
            .map(|value| {
                value
                    .backlinks
                    .iter()
                    .copied()
                    .filter(|backlink| !is_internal(*backlink))
                    .collect()
            })
            .unwrap_or_default();
        backlinks.sort();

        sections.insert(
            slug,
            GraphSection {
                parent,
                parent_specified,
                references,
                backlinks,
            },
        );
    }

    GraphSnapshot { sections }
}

pub(super) fn sync_optional_output(
    path: &Utf8Path,
    payload: Option<&str>,
    output_name: &str,
) -> eyre::Result<()> {
    if let Some(payload) = payload {
        write_text_atomically(path, payload, output_name)?;
    } else {
        let _ = remove_file_if_exists(path)?;
    }
    Ok(())
}

fn write_text_atomically(path: &Utf8Path, payload: &str, output_name: &str) -> eyre::Result<()> {
    environment::create_parent_dirs(path);

    let parent = path.parent().ok_or_else(|| {
        eyre!(
            "failed to resolve parent directory for {} `{}`",
            output_name,
            path
        )
    })?;
    let filename = path
        .file_name()
        .ok_or_else(|| eyre!("failed to resolve filename for {} `{}`", output_name, path))?;
    let temp_filename = format!(
        "{filename}.tmp.{}.{}",
        std::process::id(),
        next_atomic_write_stamp()
    );
    let temp_path = parent.join(temp_filename);

    let write_result = (|| -> eyre::Result<()> {
        let mut file = std::fs::File::create(temp_path.as_std_path())
            .wrap_err_with(|| eyre!("failed to create temp {} `{}`", output_name, temp_path))?;
        file.write_all(payload.as_bytes())
            .wrap_err_with(|| eyre!("failed to write temp {} `{}`", output_name, temp_path))?;
        file.sync_all()
            .wrap_err_with(|| eyre!("failed to sync temp {} `{}`", output_name, temp_path))?;
        Ok(())
    })();

    if let Err(err) = write_result {
        let _ = std::fs::remove_file(temp_path.as_std_path());
        return Err(err);
    }

    std::fs::rename(temp_path.as_std_path(), path.as_std_path()).wrap_err_with(|| {
        eyre!(
            "failed to atomically replace {} `{}` from `{}`",
            output_name,
            path,
            temp_path
        )
    })?;
    Ok(())
}

fn next_atomic_write_stamp() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let sequence = ARTIFACT_ATOMIC_WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("{nanos}-{sequence}")
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs};

    use super::*;
    use crate::{
        compiler::section::{
            EmbedContent, HTMLContent, LazyContent, LocalLink, SectionOption, UnresolvedSection,
        },
        entry::{HTMLMetaData, KEY_ASREF, KEY_EXT, KEY_PAGE_TITLE, KEY_SLUG, KEY_TITLE},
        ordered_map::OrderedMap,
    };

    fn shallow(slug: &str, content: HTMLContent) -> UnresolvedSection {
        let mut metadata = OrderedMap::new();
        metadata.insert(KEY_SLUG.to_string(), HTMLContent::Plain(slug.to_string()));
        metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain("md".to_string()));
        metadata.insert(KEY_TITLE.to_string(), HTMLContent::Plain(slug.to_string()));
        metadata.insert(
            KEY_PAGE_TITLE.to_string(),
            HTMLContent::Plain(slug.to_string()),
        );
        UnresolvedSection {
            metadata: HTMLMetaData(metadata),
            content,
        }
    }

    #[test]
    fn test_graph_snapshot_contains_sorted_full_graph_relationships() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("index"),
            shallow(
                "index",
                HTMLContent::Lazy(vec![
                    LazyContent::Local(LocalLink {
                        url: "/ref.md".to_string(),
                        text: None,
                    }),
                    LazyContent::Embed(EmbedContent {
                        url: "/child.md".to_string(),
                        title: None,
                        option: SectionOption::default(),
                    }),
                ]),
            ),
        );
        shallows.insert(
            Slug::new("a"),
            shallow(
                "a",
                HTMLContent::Lazy(vec![LazyContent::Local(LocalLink {
                    url: "/ref.md".to_string(),
                    text: None,
                })]),
            ),
        );

        let mut ref_section = shallow("ref", HTMLContent::Plain("<p>ref</p>".to_string()));
        ref_section.metadata.0.insert(
            KEY_ASREF.to_string(),
            HTMLContent::Plain("true".to_string()),
        );
        shallows.insert(Slug::new("ref"), ref_section);
        shallows.insert(
            Slug::new("child"),
            shallow("child", HTMLContent::Plain("<p>child</p>".to_string())),
        );

        let state = state::compile_all(&shallows).unwrap();
        let snapshot = graph_snapshot(&state);

        let index = snapshot.sections.get(&Slug::new("index")).unwrap();
        assert_eq!(index.references, vec![Slug::new("ref")]);

        let child = snapshot.sections.get(&Slug::new("child")).unwrap();
        assert_eq!(child.parent, Slug::new("index"));
        assert!(!child.parent_specified);

        let reference = snapshot.sections.get(&Slug::new("ref")).unwrap();
        assert_eq!(
            reference.backlinks,
            vec![Slug::new("a"), Slug::new("index")]
        );
    }

    #[test]
    fn test_sync_optional_output_writes_and_removes_artifact() {
        let base = crate::test_io::case_dir("sync-output");
        let path = base.join("publish/kodama.json");

        sync_optional_output(path.as_path(), Some("{\"ok\":true}"), "indexes").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"ok\":true}");

        sync_optional_output(path.as_path(), None, "indexes").unwrap();
        assert!(!path.exists());

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn test_sync_optional_output_overwrites_existing_artifact_atomically() {
        let base = crate::test_io::case_dir("sync-atomic");
        let path = base.join("publish/kodama.graph.json");

        sync_optional_output(path.as_path(), Some("{\"v\":1}"), "graph").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"v\":1}");

        sync_optional_output(path.as_path(), Some("{\"v\":2}"), "graph").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "{\"v\":2}");

        let _ = fs::remove_dir_all(base);
    }
}
