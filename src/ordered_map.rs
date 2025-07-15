use std::{borrow::Borrow, collections::HashMap, hash::Hash};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderedMap<K: Eq + Hash, V>(HashMap<K, (V, usize)>);

impl<K: Eq + Hash, V> OrderedMap<K, V> {
    pub fn new() -> OrderedMap<K, V> {
        OrderedMap(HashMap::new())
    }

    pub fn get<Q: Eq + Hash + ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
    {
        self.0.get(k).map(|t| &t.0)
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        let mut a: Vec<(&K, usize)> = self.0.iter().map(|(k, (_, i))| (k, *i)).collect();
        a.sort_by(|a, b| (a.1).cmp(&b.1));
        a.into_iter().map(|t| t.0)
    }

    pub fn insert(&mut self, k: K, v: V) -> Option<V> {
        let i = if let Some(t) = self.0.get(&k) {
            t.1
        } else {
            self.0.len()
        };
        self.0.insert(k, (v, i)).map(|t| t.0)
    }
}
