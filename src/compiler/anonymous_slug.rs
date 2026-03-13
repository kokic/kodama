// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kodama contributors

use std::collections::{HashMap, HashSet};

use crate::slug::{self, Slug};

pub const ANON_SUBTREE_SLUG_PREFIX: &str = ":";

/// (= catalog numbering)
pub const ANON_SUBTREE_ORDINAL_INITIAL: usize = 1;

#[derive(Default)]
pub struct AnonymousSlugState {
    anonymous_ordinals: HashMap<Slug, usize>,
}

impl AnonymousSlugState {
    pub fn allocate_with_used(
        &mut self,
        source_slug: Slug,
        used_slugs: &mut HashSet<Slug>,
    ) -> Slug {
        let ordinal = self
            .anonymous_ordinals
            .entry(source_slug)
            .or_insert(ANON_SUBTREE_ORDINAL_INITIAL);
        loop {
            let candidate = anonymous_slug_for(source_slug, *ordinal);
            *ordinal += 1;
            if used_slugs.insert(candidate) {
                return candidate;
            }
        }
    }
}

pub fn anonymous_slug_for(source_slug: Slug, ordinal: usize) -> Slug {
    let component = format!("{ANON_SUBTREE_SLUG_PREFIX}{ordinal}");
    let slug_path = format!("{source_slug}/{component}");
    slug::to_slug(slug_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anonymous_slug_for_uses_source_prefix() {
        assert_eq!(
            anonymous_slug_for(Slug::new("book/index"), ANON_SUBTREE_ORDINAL_INITIAL),
            Slug::new(format!("book/index/:{}", ANON_SUBTREE_ORDINAL_INITIAL))
        );
    }
}
