// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::collections::{HashMap, HashSet};

use crate::slug::Slug;

#[derive(Debug)]
pub struct CallbackValue {
    pub parent: Slug,
    pub is_parent_specified: bool,

    /// Used to record which sections reference the current section.
    pub backlinks: HashSet<Slug>,
}

#[derive(Debug)]
pub struct Callback(pub HashMap<Slug, CallbackValue>);

impl Callback {
    pub fn new() -> Callback {
        Callback(HashMap::new())
    }

    pub fn merge(&mut self, other: Callback) {
        other.0.into_iter().for_each(|(s, t)| self.insert(s, t));
    }

    pub fn insert(&mut self, child_slug: Slug, value: CallbackValue) {
        match self.0.get_mut(&child_slug) {
            None => {
                self.0.insert(child_slug, value);
            }
            Some(existed) => {
                existed.backlinks.extend(value.backlinks);

                if existed.is_parent_specified {
                    if value.is_parent_specified {
                        assert_eq!(existed.parent, value.parent);
                    }
                    return;
                }
                if value.is_parent_specified {
                    existed.parent = value.parent;
                    existed.is_parent_specified = true;
                    return;
                }
                if existed.parent == "index" {
                    existed.parent = value.parent;
                    return;
                }
                if value.parent != "index" && existed.parent != value.parent {
                    color_print::ceprintln!(
                        "<y>Warning: Multiple parents for `{}`: `{}` and `{}`. Using {}.</>",
                        child_slug, existed.parent, value.parent, existed.parent
                    );
                }
            }
        }
    }

    pub fn insert_parent(&mut self, child_slug: Slug, parent: Slug) {
        self.insert(
            child_slug,
            CallbackValue {
                parent,
                is_parent_specified: false,
                backlinks: HashSet::new(),
            },
        );
    }

    pub fn specify_parent(&mut self, child_slug: Slug, parent: Slug) {
        self.insert(
            child_slug,
            CallbackValue {
                parent,
                is_parent_specified: true,
                backlinks: HashSet::new(),
            },
        );
    }

    pub fn insert_backlinks<I>(&mut self, child_slug: Slug, backlinks: I)
    where
        I: IntoIterator<Item = Slug>,
    {
        self.insert(
            child_slug,
            CallbackValue {
                parent: Slug::new("index"),
                is_parent_specified: false,
                backlinks: HashSet::from_iter(backlinks),
            },
        );
    }
}
