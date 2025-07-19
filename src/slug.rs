// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::{fmt::Display, str::FromStr};

use camino::Utf8Path;
use internment::Intern;
use serde::{Deserialize, Serialize};

use crate::path_utils;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Slug(Intern<str>);

impl Slug {
    pub fn new<S: AsRef<str>>(s: S) -> Self {
        Self(s.as_ref().into())
    }

    pub fn as_str(&self) -> &'static str {
        self.0.as_ref()
    }
}

impl PartialEq<&str> for Slug {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<Slug> for &str {
    fn eq(&self, other: &Slug) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<Slug> for String {
    fn eq(&self, other: &Slug) -> bool {
        self == other.as_str()
    }
}

impl Serialize for Slug {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.0.as_ref())
    }
}

impl<'de> Deserialize<'de> for Slug {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Slug::new)
    }
}

impl Display for Slug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, Debug)]
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

pub fn to_hash_id(slug_str: &str) -> String {
    slug_str.replace("/", "-")
}

pub fn to_slug<P: AsRef<Utf8Path>>(path: P) -> Slug {
    let path = path.as_ref();
    Slug::new(path_utils::pretty_path(&path.with_extension("")))
}
