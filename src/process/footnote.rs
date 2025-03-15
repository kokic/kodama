use pulldown_cmark::{CowStr, Tag};

use crate::{html_flake, recorder::ParseRecorder};

use super::processer::Processer;

pub struct Footnote;

impl Processer for Footnote {
    fn footnote(
        &self,
        s: &CowStr<'_>,
        recorder: &mut crate::recorder::ParseRecorder,
    ) -> Option<String> {
        let name = s.to_string();
        let len = recorder.footnote_counter.len() + 1;
        let number = recorder.footnote_counter.entry(name.into()).or_insert(len);
        let back_id = get_back_id(s);
        Some(html_flake::footnote_reference(s, &back_id, *number))
    }

    fn start(&mut self, tag: &Tag<'_>, recorder: &mut ParseRecorder) {
        match tag {
            Tag::FootnoteDefinition(s) => {
                let name = s.to_string();
                let len = recorder.footnote_counter.len() + 1;
                let number = recorder.footnote_counter.entry(name.into()).or_insert(len);

                let back_href = format!("#{}", get_back_id(s));
                let html = format!(
                    r#"<div class="footnote-definition" id="{}">
  <sup class="footnote-definition-label"><a href="{}">{}</a></sup>"#,
                    s, back_href, number
                );
                recorder.push(html);
            }
            _ => (),
        }
    }
}

fn get_back_id(s: &CowStr<'_>) -> String {
    format!("{}-back", s)
}
