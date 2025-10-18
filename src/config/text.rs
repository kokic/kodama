// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Text {
    pub edit: String,
    pub toc: String,
    pub references: String,
    pub backlinks: String,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            edit: "[edit]".to_string(),
            toc: "Table of Contents".to_string(),
            references: "References".to_string(),
            backlinks: "Backlinks".to_string(),
        }
    }
}
