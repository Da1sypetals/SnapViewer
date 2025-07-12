use serde::Deserialize;
use std::fmt::Display;

// Corresponds to the Python Frame dataclass
#[derive(Deserialize, Debug)]
pub struct Frame {
    pub name: String, // function name
    pub filename: String,
    pub line: u32,
}

// Implement Display for Frame to make callstack printing cleaner
impl Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "  at {} ({}:{})", self.name, self.filename, self.line)
    }
}
