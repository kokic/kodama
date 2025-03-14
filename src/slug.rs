use std::{fmt::Display, str::FromStr};

#[derive(Debug)]
pub enum Ext {
    Markdown,
    Typst,
}

pub struct ParseExtensionError;

impl FromStr for Ext {
    type Err = ParseExtensionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "md" => Ok(Self::Markdown),
            "typst" => Ok(Self::Typst),
            _ => Err(ParseExtensionError),
        }
    }
}

impl Display for Ext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Ext::Markdown => "md",
            Ext::Typst => "typst",
        };
        write!(f, "{s}")
    }
}

pub fn to_hash_id(slug: &str) -> String {
    slug.replace("/", "-")
}

/// path to slug
pub fn to_slug(fullname: &str) -> String {
    to_slug_ext(fullname).0
}

pub fn to_slug_ext(fullname: &str) -> (String, Option<Ext>) {
    let mut slug = fullname;
    if fullname.starts_with("/") {
        slug = &slug[1..]
    } else if fullname.starts_with("./") {
        slug = &slug[2..]
    }
    let (maybe_slug, ext) = slug.rsplit_once('.').unzip();
    let slug = maybe_slug.unwrap_or(slug);
    let ext = ext.and_then(|e| e.parse().ok());
    (pretty_path(std::path::Path::new(&slug)), ext)
}

pub fn pretty_path(path: &std::path::Path) -> String {
    posix_style(clean_path(path).to_str().unwrap())
}

pub fn posix_style(s: &str) -> String {
    s.replace("\\", "/")
}

fn clean_path(path: &std::path::Path) -> std::path::PathBuf {
    let mut cleaned_path = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                cleaned_path.pop();
            }
            _ => {
                cleaned_path.push(component.as_os_str());
            }
        }
    }
    cleaned_path
}

pub fn adjust_name(path: &str, expect: &str, target: &str) -> String {
    let prefix = if path.ends_with(expect) {
        &path[0..path.len() - expect.len()]
    } else {
        path
    };
    format!("{}{}", prefix, target)
}
