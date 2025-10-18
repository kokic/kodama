// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
pub struct Serve {
    pub edit: Option<String>,
    pub output: String,
    pub command: Vec<String>,
}

impl Default for Serve {
    fn default() -> Self {
        Self {
            edit: Some("vscode://file/".to_string()),
            output: "./.cache/publish".to_string(),
            command: [
                "miniserve",
                "<output>",
                "--index",
                "index.html",
                "--pretty-urls",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        }
    }
}
