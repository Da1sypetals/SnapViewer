use crate::allocation::Frame;

#[derive(Debug)]
pub struct AllocationDbRow {
    pub index: usize,
    pub size: u64,
    pub callstack: String,
    pub start_time: u64,
    pub end_time: u64,
}

pub fn format_callstack(frames: &[Frame]) -> String {
    frames
        .iter()
        .map(|frame| format!("{}:{}:{}", frame.filename, frame.line, frame.name))
        .collect::<Vec<String>>()
        .join("\n")
}
