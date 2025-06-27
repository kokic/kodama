use std::path::{Path, PathBuf};

use internment::Intern;

use crate::config;

/// This structure is used to associate a section path with the corresponding hash and entry file. 
/// 
/// Related methods [`SectionPath::hash_path`], [`SectionPath::entry_path`] will not automatically create parent folders. 
pub struct SectionPath(Intern<str>);

impl SectionPath {
    pub fn new<S: AsRef<str>>(s: S) -> Self {
        Self(s.as_ref().into())
    }

    pub fn hash_path(&self) -> PathBuf {
        config::hash_dir().join(self.as_path())
    }

    pub fn as_path(&self) -> &Path {
        self.as_str().as_ref()
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}