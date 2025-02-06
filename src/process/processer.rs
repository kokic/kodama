use std::collections::HashMap;

use crate::{compiler::section::LazyContent, recorder::ParseRecorder};
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
        metadata: &mut HashMap<String, String>,
    ) {
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
    fn inline_html(
        &self,
        s: &CowStr<'_>,
        recorder: &mut ParseRecorder,
        metadata: &mut HashMap<String, String>,
    ) {
    }
}

pub fn url_action(dest_url: &CowStr<'_>) -> (String, String) {
    let vec: Vec<&str> = dest_url.split("#:").collect();
    (
        vec.first().unwrap_or(&"").to_string(),
        vec.last().unwrap_or(&"").to_string(),
    )
}
