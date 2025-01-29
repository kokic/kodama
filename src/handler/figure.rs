use pulldown_cmark::{Tag, TagEnd};

use crate::recorder::{ParseRecorder, State};

use super::Handler;

pub struct Figure;

impl Handler for Figure {
    fn start(&mut self, tag: &Tag<'_>, recorder: &mut ParseRecorder) {
        match tag {
            Tag::Image {
                link_type: _,
                dest_url,
                title: _,
                id: _,
            } => {
                recorder.enter(State::Figure);
                recorder.push(dest_url.to_string()); // [0]
            }
            _ => (),
        }
    }

    fn end(&mut self, _tag: &TagEnd, recorder: &mut ParseRecorder, _history: &mut Vec<String>) -> Option<String> {
        if recorder.state == State::Figure {
            let url = recorder.data.get(0).unwrap();
            let alt = recorder.data.get(1).unwrap();
            let html = format!(r#"<img src={} title={} alt={}>"#, url, alt, alt);
            recorder.exit();
            return Some(html);
        }
        None
    }

    fn text(
        &self,
        s: &pulldown_cmark::CowStr<'_>,
        recorder: &mut ParseRecorder,
        _metadata: &mut std::collections::HashMap<String, String>,
        _history: &mut Vec<String>
    ) {
        if recorder.state == State::Figure {
            recorder.push(s.to_string()); // [1]: alt text
        }
    }
}
