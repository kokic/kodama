use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum Context {
    None,
    Embed,
    InlineTypst, // typst
    ImageSpan,   // display: inline
    ImageBlock,  // display: block; text-align: center
    Metadata,

    LocalLink,
    ExternalLink,
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
            Context::LocalLink => "local",
            Context::ExternalLink => "external",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CatalogItem {
    pub slug: String,
    pub text: String,
    pub taxon: String, 
    pub number: bool, 
    pub summary: bool,  
    pub children: Vec<Box<CatalogItem>>,
}

pub type Catalog = Vec<Box<CatalogItem>>;

#[derive(Debug)]
pub struct Recorder {
    pub context: Context,
    pub data: Vec<String>,
    pub relative_dir: String,
    pub catalog: Catalog,
    pub taxon_map: HashMap<String, String>, 
}

impl Recorder {
    pub fn new(relative_dir: &str) -> Recorder {
        return Recorder {
            context: Context::None,
            data: vec![],
            relative_dir: relative_dir.to_string(),
            catalog: vec![],
            taxon_map: HashMap::new(), 
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Counter {
    pub numbers: Vec<u8>,
}

impl Counter {
    pub fn init() -> Self {
        return Counter { numbers: vec![0] };
    }

    pub fn display(&self) -> String {
        self.numbers
            .iter()
            .map(|n| format!("{}.", n))
            .reduce(|s: String, t| s + &t)
            .unwrap()
    }

    pub fn step_at_mut(&mut self, level: usize) {
        let len = self.numbers.len();
        let index = len - level;
        if index < len {
            self.numbers[index] += 1;
        }
    }

    pub fn step_mut(&mut self) {
        self.step_at_mut(1)
    }

    pub fn left_shift_by(&self, n: u8) -> Counter {
        let mut counter = self.clone();
        counter.numbers.push(n);
        return counter;
    }

    pub fn left_shift(&self) -> Counter {
        self.left_shift_by(0)
    }
}
