// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::collections::{HashMap, HashSet};

use crate::slug::Slug;

#[derive(Debug)]
pub struct CallbackValue {
    pub parent: Slug,

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
        match self.0.get(&child_slug) {
            None => {
                self.0.insert(child_slug, value);
            }
            Some(_) => {
                let mut existed = self.0.remove(&child_slug).unwrap();
                existed.backlinks.extend(value.backlinks);

                if existed.parent == "index" && value.parent != "index" {
                    existed.parent = value.parent;
                }
                self.0.insert(child_slug, existed);
            }
        }
    }

    pub fn insert_parent(&mut self, child_slug: Slug, parent: Slug) {
        self.insert(
            child_slug,
            CallbackValue {
                parent,
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
                backlinks: HashSet::from_iter(backlinks),
            },
        );
    }
}
