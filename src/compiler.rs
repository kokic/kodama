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

use std::{collections::HashMap, fs::File, io::BufReader};
use std::collections::{HashSet, VecDeque};

use camino::{Utf8Path, Utf8PathBuf};
use eyre::{bail, eyre, WrapErr};
use parser::parse_markdown;
use section::{HTMLContent, ShallowSection};
use typst::parse_typst;
use walkdir::WalkDir;
use writer::Writer;

use crate::{
    environment::{self, verify_and_file_hash},
    ordered_map::OrderedMap,
    path_utils,
    slug::{Ext, Slug},
};

pub type DirtySet = HashSet<Utf8PathBuf>;

fn source_relative_path(slug: Slug, ext: Ext) -> Utf8PathBuf {
    Utf8PathBuf::from(format!("{}.{}", slug, ext))
}

fn source_from_entry_relative_path(entry_relative_path: &Utf8Path) -> Option<(Utf8PathBuf, Slug, Ext)> {
    let entry_relative_path = path_utils::pretty_path(entry_relative_path);
    let source_relative_path = entry_relative_path.strip_suffix(".entry")?;
    let source_relative_path = Utf8PathBuf::from(source_relative_path);
    let ext = source_relative_path.extension()?.parse().ok()?;
    let slug = Slug::new(path_utils::pretty_path(&source_relative_path.with_extension("")));
    Some((source_relative_path, slug, ext))
}

fn same_ext(a: Ext, b: Ext) -> bool {
    matches!((a, b), (Ext::Markdown, Ext::Markdown) | (Ext::Typst, Ext::Typst))
}

fn hash_cache_path_no_create(hash_dir: &Utf8Path, source_relative_path: &Utf8Path) -> Utf8PathBuf {
    let mut hash_path = hash_dir.join(source_relative_path);
    let ext = hash_path
        .extension()
        .map(|ext| format!("{ext}.hash"))
        .unwrap_or_else(|| "hash".to_string());
    hash_path.set_extension(ext);
    hash_path
}

fn remove_file_if_exists(path: &Utf8Path) -> eyre::Result<bool> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err).wrap_err_with(|| eyre!("failed to remove file `{}`", path)),
    }
}

fn cleanup_stale_slug_artifacts_with_paths(
    workspace: &Workspace,
    entry_dir: &Utf8Path,
    hash_dir: &Utf8Path,
    output_dir: &Utf8Path,
) -> eyre::Result<HashSet<Slug>> {
    let mut stale_slugs = HashSet::new();
    if !entry_dir.exists() {
        return Ok(stale_slugs);
    }

    for entry in WalkDir::new(entry_dir).follow_links(true).into_iter() {
        let std_path = entry
            .wrap_err_with(|| eyre!("failed to read cached entry directory `{}`", entry_dir))?
            .into_path();
        let entry_path = match Utf8PathBuf::from_path_buf(std_path) {
            Ok(path) => path,
            Err(non_utf8) => {
                color_print::ceprintln!(
                    "<y>Warning: skipping non-UTF-8 cache path `{}`.</>",
                    non_utf8.display()
                );
                continue;
            }
        };
        if !entry_path.is_file() || entry_path.extension() != Some("entry") {
            continue;
        }

        let relative_entry = entry_path.strip_prefix(entry_dir).unwrap_or(entry_path.as_path());
        let Some((source_relative, slug, ext)) = source_from_entry_relative_path(relative_entry) else {
            continue;
        };

        if workspace
            .slug_exts
            .get(&slug)
            .copied()
            .is_some_and(|current_ext| same_ext(current_ext, ext))
        {
            continue;
        }

        stale_slugs.insert(slug);

        let _ = remove_file_if_exists(entry_path.as_path())?;

        let hash_path = hash_cache_path_no_create(hash_dir, source_relative.as_path());
        let _ = remove_file_if_exists(hash_path.as_path())?;

        let output_html = output_dir.join(format!("{}.html", slug));
        let _ = remove_file_if_exists(output_html.as_path())?;
    }

    Ok(stale_slugs)
}

fn cleanup_stale_slug_artifacts(workspace: &Workspace) -> eyre::Result<HashSet<Slug>> {
    cleanup_stale_slug_artifacts_with_paths(
        workspace,
        environment::entry_dir().as_path(),
        environment::hash_dir().as_path(),
        environment::output_dir().as_path(),
    )
}

fn is_source_modified(relative_path: &Utf8Path, dirty_paths: Option<&DirtySet>) -> eyre::Result<bool> {
    if *crate::cli::build::enable_no_cache() {
        return Ok(true);
    }

    if let Some(dirty_paths) = dirty_paths {
        if dirty_paths.contains(relative_path) {
            // Keep hash baseline updated for subsequent cold builds.
            let _ = verify_and_file_hash(relative_path)?;
            return Ok(true);
        }
        return Ok(false);
    }

    verify_and_file_hash(relative_path)
}

pub fn expand_dirty_paths(workspace: &Workspace, dirty_paths: &DirtySet) -> DirtySet {
    let mut expanded = dirty_paths.clone();

    let mut dirty_all_sources = false;
    let mut dirty_all_typst_sources = false;
    for path in dirty_paths {
        match path.extension() {
            Some("md") => {}
            Some("typst") | Some("typ") => {
                dirty_all_typst_sources = true;
            }
            _ => {
                // Unknown tree-side dependency (e.g. include file): conservatively reparse all.
                dirty_all_sources = true;
            }
        }
    }

    if dirty_all_sources {
        workspace
            .slug_exts
            .iter()
            .for_each(|(&slug, &ext)| {
                expanded.insert(source_relative_path(slug, ext));
            });
        return expanded;
    }

    if dirty_all_typst_sources {
        workspace
            .slug_exts
            .iter()
            .filter(|(_, &ext)| matches!(ext, Ext::Typst))
            .for_each(|(&slug, &ext)| {
                expanded.insert(source_relative_path(slug, ext));
            });
    }

    expanded
}

fn dirty_source_slugs(workspace: &Workspace, dirty_paths: &DirtySet) -> HashSet<Slug> {
    workspace
        .slug_exts
        .iter()
        .filter_map(|(&slug, &ext)| {
            let relative = source_relative_path(slug, ext);
            dirty_paths.contains(relative.as_path()).then_some(slug)
        })
        .collect()
}

fn affected_slugs_from_dirty(
    state: &state::CompileState,
    dirty_source_slugs: &HashSet<Slug>,
) -> HashSet<Slug> {
    let mut affected = dirty_source_slugs.clone();
    let mut queue: VecDeque<Slug> = dirty_source_slugs.iter().copied().collect();

    // If a linker page changes, the target's backlink list changes too.
    for (&target_slug, callback) in &state.callback().0 {
        if callback
            .backlinks
            .iter()
            .any(|backlink_slug| dirty_source_slugs.contains(backlink_slug))
            && affected.insert(target_slug)
        {
            queue.push_back(target_slug);
        }
    }

    while let Some(slug) = queue.pop_front() {
        let Some(callback) = state.callback().0.get(&slug) else {
            continue;
        };

        if callback.parent != slug && affected.insert(callback.parent) {
            queue.push_back(callback.parent);
        }

        for &backlink_slug in &callback.backlinks {
            if affected.insert(backlink_slug) {
                queue.push_back(backlink_slug);
            }
        }
    }

    affected
}

pub fn compile(workspace: Workspace, dirty_paths: Option<&DirtySet>) -> eyre::Result<()> {
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
            let shallow: ShallowSection = serde_json::from_reader(entry_file)
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

    let indexes: HashMap<Slug, OrderedMap<String, HTMLContent>> = shallows
        .iter()
        .map(|(slug, section)| (*slug, section.metadata.0.clone()))
        .collect();

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

    let serialized = serde_json::to_string(&indexes)
        .wrap_err_with(|| eyre!("failed to serialize indexes to JSON"))?;
    let indexes_path = environment::indexes_path(&environment::output_dir());
    std::fs::write(&indexes_path, serialized)
        .wrap_err_with(|| eyre!("failed to write indexes to `{}`", indexes_path))?;

    Ok(())
}

pub fn should_ignored_file(path: &Utf8Path) -> bool {
    path.file_name().is_some_and(|name| name == "README.md")
}

pub fn should_ignored_dir(path: &Utf8Path) -> bool {
    path.file_name()
        .is_some_and(|name| name.starts_with(['.', '_']))
}

fn to_slug_ext(source_dir: &Utf8Path, p: &Utf8Path) -> Option<(Slug, Ext)> {
    let p = p.strip_prefix(source_dir).unwrap_or(p);
    let ext = p.extension()?.parse().ok()?;
    let slug = Slug::new(path_utils::pretty_path(&p.with_extension("")));
    Some((slug, ext))
}

/// Collect all source file paths in `<trees>` dir.
///
/// **Side effect: update the `.hash` & `.svg` file of all modified `.typ` files.**
pub fn all_trees_source(trees_dir: &Utf8Path, dirty_paths: Option<&DirtySet>) -> eyre::Result<Workspace> {
    let mut slug_exts = HashMap::new();

    let failed_to_read_dir = |dir: &Utf8Path| eyre!("failed to read directory `{}`", dir);
    let file_collide = |p: &Utf8Path, e: Ext| {
        eyre!(
            "`{}` collides with `{}`",
            p,
            p.with_extension(e.to_string()),
        )
    };

    let mut collect_files = |source_dir: &Utf8Path| {
        let compile_typst_svg = |path: &Utf8PathBuf| -> eyre::Result<()> {
            // Hashable files only include `.md` and `.typ` currently.
            if let Some("typ") = path.extension() {
                let relative = path.strip_prefix(source_dir)?;
                if let Some(dirty_paths) = dirty_paths {
                    if !dirty_paths.contains(relative) {
                        return Ok(());
                    }
                }

                let svg_url = relative.with_extension("svg");
                let svg_path = environment::output_path(&svg_url);
                if let Err(err) = crate::typst_cli::write_svg(relative, &svg_path) {
                    color_print::ceprintln!("<r>{:?} at {}</>", err, path);
                }
            }
            Ok(())
        };

        for entry in source_dir
            .read_dir_utf8()
            .wrap_err_with(|| failed_to_read_dir(source_dir))?
        {
            let path = entry
                .wrap_err_with(|| failed_to_read_dir(source_dir))?
                .into_path();

            if path.is_file() && !should_ignored_file(&path) {
                let Some((slug, ext)) = to_slug_ext(source_dir, &path) else {
                    compile_typst_svg(&path)?;
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
                        Utf8Path::from_path(e.path())
                            .is_some_and(|p| p.is_file() || !should_ignored_dir(p))
                    })
                {
                    let std_path = entry.wrap_err_with(|| failed_to_read_dir(&path))?.into_path();
                    let path = match Utf8PathBuf::from_path_buf(std_path) {
                        Ok(path) => path,
                        Err(non_utf8) => {
                            color_print::ceprintln!(
                                "<y>Warning: skipping non-UTF-8 path `{}`.</>",
                                non_utf8.display()
                            );
                            continue;
                        }
                    };
                    if path.is_file() {
                        let Some((slug, ext)) = to_slug_ext(source_dir, &path) else {
                            compile_typst_svg(&path)?;
                            continue;
                        };
                        if let Some(ext) = slug_exts.insert(slug, ext) {
                            bail!(file_collide(&path, ext));
                        }
                    }
                }
            }
        }
        Ok(())
    };

    if !trees_dir.exists() {
        color_print::ceprintln!(
            "<y>Warning: Source directory `{}` does not exist, skipping.</>",
            trees_dir
        );
    }

    collect_files(trees_dir)?;

    Ok(Workspace { slug_exts })
}

#[derive(Debug)]
pub struct Workspace {
    pub slug_exts: HashMap<Slug, Ext>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use crate::compiler::section::{HTMLContent, LazyContent, LocalLink, ShallowSection};
    use crate::entry::{HTMLMetaData, KEY_EXT, KEY_PAGE_TITLE, KEY_SLUG, KEY_TITLE};
    use crate::ordered_map::OrderedMap;

    fn shallow(slug: &str, content: HTMLContent) -> ShallowSection {
        let mut metadata = OrderedMap::new();
        metadata.insert(KEY_SLUG.to_string(), HTMLContent::Plain(slug.to_string()));
        metadata.insert(KEY_EXT.to_string(), HTMLContent::Plain("md".to_string()));
        metadata.insert(KEY_TITLE.to_string(), HTMLContent::Plain(slug.to_string()));
        metadata.insert(
            KEY_PAGE_TITLE.to_string(),
            HTMLContent::Plain(slug.to_string()),
        );
        ShallowSection {
            metadata: HTMLMetaData(metadata),
            content,
        }
    }

    #[test]
    fn test_should_ignored_helpers_handle_missing_file_name() {
        let empty = Utf8Path::new("");
        assert!(!should_ignored_file(empty));
        assert!(!should_ignored_dir(empty));
    }

    #[test]
    fn test_should_ignored_helpers_match_expected_names() {
        assert!(should_ignored_file(Utf8Path::new("README.md")));
        assert!(!should_ignored_file(Utf8Path::new("docs.md")));

        assert!(should_ignored_dir(Utf8Path::new(".git")));
        assert!(should_ignored_dir(Utf8Path::new("_tmp")));
        assert!(!should_ignored_dir(Utf8Path::new("trees")));
    }

    #[test]
    fn test_expand_dirty_paths_typst_dependency_marks_all_typst_sources() {
        let mut slug_exts = HashMap::new();
        slug_exts.insert(Slug::new("a"), Ext::Markdown);
        slug_exts.insert(Slug::new("b"), Ext::Typst);
        slug_exts.insert(Slug::new("c"), Ext::Typst);
        let workspace = Workspace { slug_exts };

        let mut dirty = DirtySet::new();
        dirty.insert(Utf8PathBuf::from("shared.typ"));

        let expanded = expand_dirty_paths(&workspace, &dirty);
        assert!(expanded.contains(&Utf8PathBuf::from("shared.typ")));
        assert!(expanded.contains(&Utf8PathBuf::from("b.typst")));
        assert!(expanded.contains(&Utf8PathBuf::from("c.typst")));
        assert!(!expanded.contains(&Utf8PathBuf::from("a.md")));
    }

    #[test]
    fn test_expand_dirty_paths_unknown_tree_file_marks_all_sources() {
        let mut slug_exts = HashMap::new();
        slug_exts.insert(Slug::new("a"), Ext::Markdown);
        slug_exts.insert(Slug::new("b"), Ext::Typst);
        let workspace = Workspace { slug_exts };

        let mut dirty = DirtySet::new();
        dirty.insert(Utf8PathBuf::from("includes/snippet.txt"));

        let expanded = expand_dirty_paths(&workspace, &dirty);
        assert!(expanded.contains(&Utf8PathBuf::from("a.md")));
        assert!(expanded.contains(&Utf8PathBuf::from("b.typst")));
    }

    #[test]
    fn test_dirty_source_slugs_maps_relative_paths_to_slug_ids() {
        let mut slug_exts = HashMap::new();
        slug_exts.insert(Slug::new("a"), Ext::Markdown);
        slug_exts.insert(Slug::new("b"), Ext::Typst);
        let workspace = Workspace { slug_exts };

        let mut dirty = DirtySet::new();
        dirty.insert(Utf8PathBuf::from("a.md"));
        dirty.insert(Utf8PathBuf::from("unknown.txt"));

        let dirty_slugs = dirty_source_slugs(&workspace, &dirty);
        assert!(dirty_slugs.contains(&Slug::new("a")));
        assert!(!dirty_slugs.contains(&Slug::new("b")));
    }

    #[test]
    fn test_affected_slugs_include_link_targets_when_linker_changes() {
        let mut shallows = HashMap::new();
        shallows.insert(
            Slug::new("a"),
            shallow(
                "a",
                HTMLContent::Lazy(vec![LazyContent::Local(LocalLink {
                    url: "/b.md".to_string(),
                    text: None,
                })]),
            ),
        );
        shallows.insert(
            Slug::new("b"),
            shallow("b", HTMLContent::Plain("<p>b</p>".to_string())),
        );

        let state = state::compile_all(shallows).unwrap();
        let dirty_slugs = HashSet::from([Slug::new("a")]);
        let affected = affected_slugs_from_dirty(&state, &dirty_slugs);

        assert!(affected.contains(&Slug::new("a")));
        assert!(affected.contains(&Slug::new("b")));
    }

    #[test]
    fn test_source_from_entry_relative_path_parses_slug_and_extension() {
        let relative = Utf8Path::new("foo/bar.md.entry");
        let (source_relative, slug, ext) = source_from_entry_relative_path(relative).unwrap();
        assert_eq!(source_relative, Utf8PathBuf::from("foo/bar.md"));
        assert_eq!(slug, Slug::new("foo/bar"));
        assert!(matches!(ext, Ext::Markdown));
    }

    #[test]
    fn test_cleanup_stale_slug_artifacts_removes_stale_output_and_cache() {
        let base = std::env::temp_dir().join(format!("kodama-cleanup-{}", fastrand::u64(..)));
        let base = Utf8PathBuf::from_path_buf(base).unwrap();
        let entry_dir = base.join("entry");
        let hash_dir = base.join("hash");
        let output_dir = base.join("output");
        fs::create_dir_all(&entry_dir).unwrap();
        fs::create_dir_all(&hash_dir).unwrap();
        fs::create_dir_all(&output_dir).unwrap();

        let stale_source = Utf8PathBuf::from("old.md");
        let mut stale_entry = entry_dir.join(&stale_source);
        stale_entry.set_extension("md.entry");
        let stale_hash = hash_cache_path_no_create(hash_dir.as_path(), stale_source.as_path());
        let stale_output = output_dir.join("old.html");
        fs::create_dir_all(stale_entry.parent().unwrap()).unwrap();
        fs::create_dir_all(stale_hash.parent().unwrap()).unwrap();
        fs::write(&stale_entry, "{}").unwrap();
        fs::write(&stale_hash, "1").unwrap();
        fs::write(&stale_output, "<html/>").unwrap();

        let keep_source = Utf8PathBuf::from("keep.md");
        let mut keep_entry = entry_dir.join(&keep_source);
        keep_entry.set_extension("md.entry");
        let keep_hash = hash_cache_path_no_create(hash_dir.as_path(), keep_source.as_path());
        let keep_output = output_dir.join("keep.html");
        fs::write(&keep_entry, "{}").unwrap();
        fs::write(&keep_hash, "1").unwrap();
        fs::write(&keep_output, "<html/>").unwrap();

        let mut slug_exts = HashMap::new();
        slug_exts.insert(Slug::new("keep"), Ext::Markdown);
        let workspace = Workspace { slug_exts };

        let stale = cleanup_stale_slug_artifacts_with_paths(
            &workspace,
            entry_dir.as_path(),
            hash_dir.as_path(),
            output_dir.as_path(),
        )
        .unwrap();

        assert!(stale.contains(&Slug::new("old")));
        assert!(!stale.contains(&Slug::new("keep")));
        assert!(!stale_entry.exists());
        assert!(!stale_hash.exists());
        assert!(!stale_output.exists());
        assert!(keep_entry.exists());
        assert!(keep_hash.exists());
        assert!(keep_output.exists());

        let _ = fs::remove_dir_all(base);
    }
}
