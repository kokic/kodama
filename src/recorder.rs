use std::collections::HashMap;

use crate::process::processer::FootnoteCounter;


#[derive(Debug, PartialEq)]
pub enum State {
    /// Writable state
    None,
    Embed,

    /// Shared for inline typst
    Shared,

    /// Export typst to HTML fragment
    Html, 

    /// Inline typst
    InlineTypst, 

    /// `display: inline`
    ImageSpan,

    /// `display: block; text-align: center`
    ImageBlock,

    /// `ImageBlock` with `<details>` code
    ImageCode,

    Metadata,
    Figure,
    LocalLink,
    ExternalLink,
}

impl State {
    pub const fn strify(&self) -> &'static str {
        match self {
            State::None => "none",
            State::Embed => "embed",
            State::Shared => "shared",
            State::Html => "html", 
            State::InlineTypst => "inline",
            State::ImageSpan => "span",
            State::ImageBlock => "block",
            State::ImageCode => "code",
            State::Metadata => "metadata",
            State::Figure => "figure",
            State::LocalLink => "local",       // style class name
            State::ExternalLink => "external", // style class name
        }
    }
}

#[derive(Debug)]
pub struct ParseRecorder {
    pub state: State,
    pub current: String,
    pub data: Vec<String>,
    pub shareds: Vec<String>,
    pub footnote_counter: FootnoteCounter
}

impl ParseRecorder {
    pub fn new(current: String) -> ParseRecorder {
        return ParseRecorder {
            state: State::None,
            current,
            data: vec![],
            shareds: vec![],
            footnote_counter: HashMap::new(), 
        };
    }

    pub fn enter(&mut self, form: State) {
        self.state = form;
    }

    pub fn exit(&mut self) {
        self.state = State::None;
        self.data.clear();
    }

    pub fn push(&mut self, s: String) {
        self.data.push(s);
    }

    pub fn is_html_writable(&self) -> bool {
        matches!(self.state, State::None)
    }
}
