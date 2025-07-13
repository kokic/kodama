// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use pulldown_cmark::Event;

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
        self.events.next().map(|e| match e {
            Event::DisplayMath(math) => Event::DisplayMath(formula_disambiguate(&math).into()),
            Event::InlineMath(math) => Event::InlineMath(formula_disambiguate(&math).into()),
            _ => e,
        })
    }
}

/// Replace the formula `<` with `< ` to avoid HTML syntax issues when parsing `<`.
fn formula_disambiguate(s: &str) -> String {
    s.replace("<", " < ")
}
