use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, clap::ValueEnum, Default, Deserialize, Serialize)]
pub enum TocPlacement {
    #[serde(rename = "left")]
    Left,

    #[default]
    #[serde(rename = "right")]
    Right,
}

#[derive(Deserialize, Debug, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Toc {
    pub placement: TocPlacement,
    pub sticky: bool,
    pub mobile_sticky: bool,
    pub max_width: String,
}

impl Default for Toc {
    fn default() -> Self {
        Self {
            placement: TocPlacement::Right,
            sticky: true,
            mobile_sticky: true,
            max_width: "45ex".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct ParseTocPlacementError;

impl FromStr for TocPlacement {
    type Err = ParseTocPlacementError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "left" => Ok(TocPlacement::Left),
            "right" => Ok(TocPlacement::Right),
            _ => Err(ParseTocPlacementError),
        }
    }
}

impl std::fmt::Display for TocPlacement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TocPlacement::Left => write!(f, "left"),
            TocPlacement::Right => write!(f, "right"),
        }
    }
}
