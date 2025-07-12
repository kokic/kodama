// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use pulldown_cmark::{CowStr, Event};

use crate::recorder::{ParseRecorder, State};

use super::processer::Processer;

pub struct KatexCompact;

pub struct KatexCompat2<E> {
    events: E,
}

impl<E> KatexCompat2<E> {
    pub fn new(events: E) -> Self {
        Self { events }
    }
}

impl<'e, E: Iterator<Item = Event<'e>>> Iterator for KatexCompat2<E> {
    type Item = Event<'e>;

    fn next(&mut self) -> Option<Self::Item> {
        // TODO: see if inline math is actually needed
        self.events.next().map(|e| {
            match e {
                Event::DisplayMath(math) => Event::DisplayMath(formula_disambiguate(&math).into()),
                Event::InlineMath(math) => Event::InlineMath(formula_disambiguate(&math).into()),
                _ => e,
            }
        })
    }
}

/// Replace the formula `<` with `< ` to avoid HTML syntax issues when parsing `<`.
fn formula_disambiguate(s: &str) -> String {
    s.replace("<", " < ")
}

impl Processer for KatexCompact {
    fn inline_math(
        &self,
        s: &pulldown_cmark::CowStr<'_>,
        recorder: &mut ParseRecorder,
    ) -> Option<std::string::String> {
        match recorder.state {
            State::InlineTypst => {
                let inline_typst = format!("${}$", s);
                recorder.push(inline_typst);
                None
            }
            _ => Some(format!("${}$", formula_disambiguate(&s))),
        }
    }

    fn display_math(&self, s: &CowStr<'_>, _recorder: &mut ParseRecorder) -> Option<String> {
        Some(format!("$${}$$", formula_disambiguate(&s)))
    }
}
