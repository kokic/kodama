// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use pulldown_cmark::CowStr;

/// split `url#:action` to `(url, action)`
pub fn url_action(dest_url: &CowStr<'_>) -> (String, String) {
    if let Some(pos) = dest_url.find("#:") {
        let base = &dest_url[0..pos];
        let action = &dest_url[pos + 2..];
        (base.to_string(), action.to_string())
    } else {
        (dest_url.to_string(), String::new())
    }
}
