#[derive(Debug, PartialEq)]
pub enum Context {
    None,
    Embed,
    InlineTypst, // typst 
    ImageSpan,  // display: inline
    ImageBlock, // display: block; text-align: center
    Metadata,
}

impl Context {
    pub fn strify(&self) -> &str {
        match self {
            Context::None => "none",
            Context::Embed => "embed",
            Context::InlineTypst => "inline",
            Context::ImageSpan => "span",
            Context::ImageBlock => "block",
            Context::Metadata => "metadata",
        }
    }
}

#[derive(Debug)]
pub struct Recorder {
    pub context: Context,
    pub data: Vec<String>,
    pub relative_dir: String,
    pub catalog: Vec<(String, String)>,
}

impl Recorder {
    pub fn new(relative_dir: &str) -> Recorder {
        return Recorder {
            context: Context::None,
            data: vec![],
            relative_dir: relative_dir.to_string(),
            catalog: vec![],
        };
    }

    pub fn enter(&mut self, form: Context) {
        self.context = form;
    }

    pub fn exit(&mut self) {
        self.context = Context::None;
        self.data.clear();
    }

    pub fn push(&mut self, s: String) {
        self.data.push(s);
    }

    pub fn is_none(&self) -> bool {
        matches!(self.context, Context::None)
    }
}