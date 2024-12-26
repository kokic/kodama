use pulldown_cmark::CowStr;

use crate::recorder::Context;

use super::Handler;

pub struct KatexCompact;

/// Replace the formula `<` with `< ` to avoid HTML syntax issues when parsing `<`.
fn formula_disambiguate(s: &str) -> String {
    s.replace("<", "< ")
}

impl Handler for KatexCompact {
    
    fn inline_math(
        &self,
        s: &pulldown_cmark::CowStr<'_>,
        recorder: &mut crate::recorder::Recorder,
    ) -> Option<std::string::String> {
        match recorder.context {
            Context::InlineTypst => {
                let inline_typst = format!("${}$", s);
                recorder.push(inline_typst);
                None
            }
            _ => Some(format!("${}$", formula_disambiguate(&s))),
        }
    }

    fn display_math(&self, s: &CowStr<'_>, _recorder: &mut crate::recorder::Recorder) -> Option<String> {
        Some(format!("$${}$$", formula_disambiguate(&s)))
    }

}
