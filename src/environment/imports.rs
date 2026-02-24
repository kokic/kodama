// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::fs;

const DEFAULT_IMPORT_FONT_HTML: &str = include_str!("../include/import-font.html");
const DEFAULT_IMPORT_MATH_HTML: &str = include_str!("../include/import-math.html");

pub fn import_meta_html() -> String {
    fs::read_to_string(super::root_dir().join("import-meta.html")).unwrap_or_default()
}

pub fn import_style_html() -> String {
    fs::read_to_string(super::root_dir().join("import-style.html")).unwrap_or_default()
}

pub fn import_fonts_html() -> String {
    fs::read_to_string(super::root_dir().join("import-font.html"))
        .unwrap_or_else(|_| DEFAULT_IMPORT_FONT_HTML.to_string())
}

pub fn import_math_html() -> String {
    fs::read_to_string(super::root_dir().join("import-math.html"))
        .unwrap_or_else(|_| DEFAULT_IMPORT_MATH_HTML.to_string())
}
