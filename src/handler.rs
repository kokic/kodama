pub mod embed_markdown;
pub mod katex_compat;
pub mod typst_image;

use std::collections::HashMap;

use crate::recorder::Recorder;
use pulldown_cmark::{CowStr, Tag, TagEnd};

pub trait Handler {
    #[allow(unused_variables)]
    fn start(&mut self, tag: &Tag<'_>, recorder: &mut Recorder) {}

    #[allow(unused_variables)]
    fn end(&mut self, tag: &TagEnd, recorder: &mut Recorder) -> Option<String> {
        None
    }

    #[allow(dead_code, unused_variables)]
    fn text(&self, s: &CowStr<'_>, recorder: &mut Recorder, metadata: &mut HashMap<String, String>) {}

    #[allow(dead_code, unused_variables)]
    fn inline_math(&self, s: &CowStr<'_>, recorder: &mut Recorder) -> Option<String> {
        None
    }

    #[allow(dead_code, unused_variables)]
    fn display_math(&self, s: &CowStr<'_>, recorder: &mut Recorder) -> Option<String> {
        None
    }
}

pub fn url_action(dest_url: &CowStr<'_>) -> (String, String) {
    let vec: Vec<&str> = dest_url.split("#:").collect();
    (
        vec.first().unwrap_or(&"").to_string(),
        vec.last().unwrap_or(&"").to_string(),
    )
}
