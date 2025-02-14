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
        other.0.into_iter().for_each(|(s, t)| self.insert(s, t));
    }

    pub fn insert(&mut self, child_slug: String, value: CallbackValue) {
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
                self.0.insert(child_slug.to_string(), existed);
            }
        }
    }

    pub fn insert_parent(&mut self, child_slug: String, parent: String) {
        self.insert(
            child_slug,
            CallbackValue {
                parent,
                backlinks: HashSet::new(),
            },
        );
    }

    pub fn insert_backlinks<I>(&mut self, child_slug: String, backlinks: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.insert(
            child_slug,
            CallbackValue {
                parent: "index".to_string(),
                backlinks: HashSet::from_iter(backlinks),
            },
        );
    }
}
