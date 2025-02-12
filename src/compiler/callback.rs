use std::collections::{HashMap, HashSet};

/// We do not consider the `Context` and `Related` in the concept of forest here,
/// because to some extent, the function of `Context` has been merged into the `nav`
/// at the beginning of the page. And the concept of `Related` often serves
/// the same function as the table of contents.
#[derive(Debug)]
pub struct CallbackValue {
    pub parent: String,
    pub backlinks: HashSet<String>,
}

#[derive(Debug)]
pub struct Callback(pub HashMap<String, CallbackValue>);

impl Callback {
    pub fn new() -> Callback {
        Callback(HashMap::new())
    }

    pub fn merge(&mut self, other: Callback) {
        other
            .0
            .into_iter()
            .for_each(|(s, t)| self.insert(s, t));
    }

    pub fn insert(&mut self, child_slug: String, value: CallbackValue) {
        self.insert_backlinks(child_slug, value.backlinks);
    }

    pub fn insert_parent(&mut self, child_slug: String, parent: String) {
        match self.0.remove(&child_slug) {
            None => {
                self.0.insert(
                    child_slug,
                    CallbackValue {
                        parent,
                        backlinks: HashSet::new(),
                    },
                );
            }
            Some(mut existed) => {
                existed.parent = parent;
                self.0.insert(child_slug, existed);
            }
        }
    }

    pub fn insert_backlinks<I>(&mut self, child_slug: String, backlinks: I)
    where
        I: IntoIterator<Item = String>,
    {
        match self.0.remove(&child_slug) {
            None => {
                self.0.insert(
                    child_slug,
                    CallbackValue {
                        parent: "index".to_string(),
                        backlinks: HashSet::from_iter(backlinks),
                    },
                );
            }
            Some(mut existed) => {
                existed.backlinks.extend(backlinks);
                self.0.insert(child_slug, existed);
            }
        }
    }
}
