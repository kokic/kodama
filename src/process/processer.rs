// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

use std::collections::HashMap;

use crate::{
    compiler::section::{HTMLContent, LazyContent},
    recorder::ParseRecorder,
};
use pulldown_cmark::{CowStr, Tag, TagEnd};

pub trait Processer {
    #[allow(unused_variables)]
    fn start(&mut self, tag: &Tag<'_>, recorder: &mut ParseRecorder) {}

    #[allow(unused_variables)]
    fn end(&mut self, tag: &TagEnd, recorder: &mut ParseRecorder) -> Option<LazyContent> {
        None
    }

    #[allow(dead_code, unused_variables)]
    fn text(
        &self,
        s: &CowStr<'_>,
        recorder: &mut ParseRecorder,
        metadata: &mut HashMap<String, HTMLContent>,
    ) -> eyre::Result<()> {
        Ok(())
    }

    #[allow(dead_code, unused_variables)]
    fn inline_math(&self, s: &CowStr<'_>, recorder: &mut ParseRecorder) -> Option<String> {
        None
    }

    #[allow(dead_code, unused_variables)]
    fn display_math(&self, s: &CowStr<'_>, recorder: &mut ParseRecorder) -> Option<String> {
        None
    }

    #[allow(dead_code, unused_variables)]
    fn inline_html(&self, s: &CowStr<'_>, recorder: &mut ParseRecorder) {}

    #[allow(dead_code, unused_variables)]
    fn code(&self, s: &CowStr<'_>, recorder: &mut ParseRecorder) {}

    #[allow(dead_code, unused_variables)]
    fn footnote(&self, s: &CowStr<'_>, recorder: &mut ParseRecorder) -> Option<String> {
        None
    }
}

pub type FootnoteCounter = HashMap<String, usize>;

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
