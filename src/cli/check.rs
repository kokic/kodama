// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::collections::{HashMap, HashSet};

use eyre::{bail, eyre, WrapErr};

use crate::{
    compiler::{
        self,
        section::{HTMLContent, LazyContent, UnresolvedSection},
    },
    config,
    environment::{self, BuildMode},
    path_utils,
    process::typst_image::{reset_typst_image_error_flag, typst_image_error_detected},
    process::embed_markdown::{include_error_detected, reset_include_error_flag},
    slug::{self, Ext, Slug},
};

#[derive(clap::Args)]
pub struct CheckCommand {
    /// Path to the configuration file (e.g., "Kodama.toml").
    #[arg(short, long, default_value_t = config::DEFAULT_CONFIG_PATH.into())]
    config: String,

    /// Treat warnings as errors.
    #[arg(long, default_value_t = false)]
    strict: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Severity {
    Error,
    Warning,
    Hint,
}

struct Diagnostic {
    severity: Severity,
    message: String,
}

impl Diagnostic {
    fn error(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
        }
    }

    fn warning(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
        }
    }

    fn hint(message: impl Into<String>) -> Self {
        Self {
            severity: Severity::Hint,
            message: message.into(),
        }
    }
}

pub fn check(command: &CheckCommand) -> eyre::Result<()> {
    environment::init_environment(command.config.clone().into(), BuildMode::Check)?;

    let trees_dir = environment::trees_dir();
    let workspace = compiler::all_trees_source_readonly(trees_dir.as_path())
        .wrap_err_with(|| eyre!("failed to scan trees dir `{}`", trees_dir))?;

    let mut diagnostics = Vec::new();
    if workspace.slug_exts.is_empty() {
        diagnostics.push(Diagnostic::hint(format!(
            "No sections found under `{}`.",
            trees_dir
        )));
    }
    if !workspace.slug_exts.contains_key(&Slug::new("index")) {
        diagnostics.push(Diagnostic::warning(
            "Missing `index` section. Add `index.md` or `index.typst`.",
        ));
    }

    reset_typst_image_error_flag();
    reset_include_error_flag();
    let shallows = parse_shallows_no_cache(&workspace, &mut diagnostics);
    if typst_image_error_detected() {
        diagnostics.push(Diagnostic::error(
            "Typst render errors were detected while elaborating markdown content.",
        ));
    }
    if include_error_detected() {
        diagnostics.push(Diagnostic::error(
            "Include file read errors were detected while elaborating markdown content.",
        ));
    }
    collect_dangling_local_links(&shallows, &workspace, &mut diagnostics);
    let has_parse_errors = diagnostics.iter().any(|d| d.severity == Severity::Error);
    if !has_parse_errors {
        validate_compile_graph(&shallows, &mut diagnostics);
    }

    for diagnostic in &diagnostics {
        print_diagnostic(diagnostic);
    }

    let errors = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Warning)
        .count();
    let hints = diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Hint)
        .count();

    let strict_note = if command.strict { " (strict mode)" } else { "" };
    println!(
        "Check result: {} error(s), {} warning(s), {} hint(s){}.",
        errors, warnings, hints, strict_note
    );

    if errors > 0 {
        bail!("check failed with {} error(s)", errors);
    }
    if command.strict && warnings > 0 {
        bail!("check failed in strict mode with {} warning(s)", warnings);
    }
    Ok(())
}

fn parse_shallows_no_cache(
    workspace: &compiler::Workspace,
    diagnostics: &mut Vec<Diagnostic>,
) -> HashMap<Slug, UnresolvedSection> {
    let mut shallows = HashMap::new();
    let mut entries: Vec<(Slug, Ext)> = workspace
        .slug_exts
        .iter()
        .map(|(&slug, &ext)| (slug, ext))
        .collect();
    entries.sort_by_key(|(slug, _)| slug.as_str());

    for (slug, ext) in entries {
        match compiler::parse_source(slug, ext) {
            Ok(section) => {
                shallows.insert(slug, section);
            }
            Err(err) => diagnostics.push(Diagnostic::error(format!(
                "Failed to parse `{slug}.{ext}`: {err:#}"
            ))),
        }
    }

    shallows
}

fn collect_dangling_local_links(
    shallows: &HashMap<Slug, UnresolvedSection>,
    workspace: &compiler::Workspace,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut seen = HashSet::new();
    for (&from_slug, section) in shallows {
        let HTMLContent::Lazy(contents) = &section.content else {
            continue;
        };
        for content in contents {
            let LazyContent::Local(local) = content else {
                continue;
            };
            let target_slug = resolve_subsection_slug(from_slug, &local.url);
            if workspace.slug_exts.contains_key(&target_slug) {
                continue;
            }
            if seen.insert((from_slug, target_slug, local.url.clone())) {
                diagnostics.push(Diagnostic::warning(format!(
                    "Dangling local link in `{}`: `{}` resolves to missing section `{}`.",
                    from_slug, local.url, target_slug
                )));
            }
        }
    }
}

fn validate_compile_graph(
    shallows: &HashMap<Slug, UnresolvedSection>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if shallows.is_empty() {
        return;
    }
    if let Err(err) = compiler::state::compile_all_without_missing_index_warning(shallows) {
        diagnostics.push(Diagnostic::error(format!(
            "Failed to compile section graph: {err:#}"
        )));
    }
}

fn resolve_subsection_slug(current_slug: Slug, url: &str) -> Slug {
    slug::to_slug(path_utils::relative_to_current(current_slug.as_str(), url))
}

fn print_diagnostic(diagnostic: &Diagnostic) {
    match diagnostic.severity {
        Severity::Error => color_print::ceprintln!("<r>Error:</> {}", diagnostic.message),
        Severity::Warning => color_print::ceprintln!("<y>Warning:</> {}", diagnostic.message),
        Severity::Hint => color_print::ceprintln!("<dim>Hint:</> {}", diagnostic.message),
    }
}
