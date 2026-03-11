// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kodama contributors

use std::collections::HashSet;

use eyre::eyre;

use crate::{
    path_utils,
    slug::{self, Slug},
};

use super::section::UnresolvedSection;

pub fn resolve_subtree_slug(current_slug: Slug, raw_slug: &str) -> eyre::Result<Slug> {
    let component = raw_slug.trim();
    if component.is_empty() {
        return Err(eyre!("slug cannot be empty"));
    }
    if component == "." || component == ".." {
        return Err(eyre!("slug must be a concrete path component name"));
    }
    if component.contains('/') || component.contains('\\') {
        return Err(eyre!(
            "slug must be a single path component name without separators"
        ));
    }

    let relative = path_utils::relative_to_current(current_slug.as_str(), component);
    Ok(slug::to_slug(relative))
}

pub fn ensure_unique_section_slugs(
    sections: &[(Slug, UnresolvedSection)],
    source_slug: Slug,
    subtree_kind: &str,
) -> eyre::Result<()> {
    let mut seen = HashSet::new();
    for (slug, _) in sections {
        if !seen.insert(*slug) {
            return Err(eyre!(
                "duplicate {subtree_kind} slug `{}` generated from `{}`",
                slug,
                source_slug
            ));
        }
    }
    Ok(())
}
