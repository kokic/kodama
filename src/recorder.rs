
#[derive(Debug, PartialEq)]
pub enum State {
    None, // writable

    Embed,
    Shared,      // shared for inline typst
    InlineTypst, // typst
    ImageSpan,   // display: inline
    ImageBlock,  // display: block; text-align: center
    Metadata,

    Figure,

    LocalLink,
    ExternalLink,
}

impl State {
    pub fn strify(&self) -> &str {
        match self {
            State::None => "none",
            State::Embed => "embed",
            State::InlineTypst => "inline",
            State::ImageSpan => "span",
            State::ImageBlock => "block",
            State::Metadata => "metadata",
            State::LocalLink => "local",       // style class name
            State::ExternalLink => "external", // style class name
            State::Shared => "shared",
            State::Figure => "figure",
        }
    }
}


#[derive(Debug)]
pub struct ParseRecorder {
    pub state: State,
    pub current: String,
    pub data: Vec<String>,
    pub shareds: Vec<String>,
}

impl ParseRecorder {
    pub fn new(current: String) -> ParseRecorder {
        return ParseRecorder {
            state: State::None,
            current,
            data: vec![],
            shareds: vec![],
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
