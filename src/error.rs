use std::fmt::Display;

#[derive(Debug)]
pub enum SSError {
    Index(usize),
    ConvertChar(u32),
    Http(String),
    Selector(String),
    Parse(String),
    Empty,
}

impl std::error::Error for SSError {}

impl Display for SSError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
