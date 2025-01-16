#[derive(Debug, PartialEq)]
pub enum State {
    None,
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

#[derive(Debug, Clone)]
pub struct CatalogItem {
    pub slug: String,
    pub text: String,
    pub taxon: String,
    pub number: bool,
    pub summary: bool,
    pub hide: bool,
    pub children: Vec<Box<CatalogItem>>,
}

pub type Catalog = Vec<Box<CatalogItem>>;

#[derive(Debug)]
pub struct Recorder {
    pub state: State,
    pub current: String, 
    pub data: Vec<String>,
    pub catalog: Catalog,
    pub shareds: Vec<String>,
}

impl Recorder {
    pub fn new(current: String) -> Recorder {
        return Recorder {
            state: State::None,
            current, 
            data: vec![],
            catalog: vec![],
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

    pub fn is_none(&self) -> bool {
        matches!(self.state, State::None)
    }
}

#[derive(Debug, Clone)]
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
