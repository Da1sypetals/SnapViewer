use crate::utils::format_bytes;
use serde::Deserialize;
use std::fmt::{Display, Formatter, Result};

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

// Corresponds to the Python Allocation dataclass
#[derive(Deserialize, Debug)]
pub struct Allocation {
    pub timesteps: Vec<u64>, // x coords, sorted
    pub offsets: Vec<u64>,   // y coords, length same as `timesteps`
    pub size: u64,           // height (sweep distance)
    pub callstack: Vec<Frame>,
    pub peak_mem: u64,
    pub peak_timestamps: Vec<u64>, // reaches its peak at these timestamps
}

impl Display for Allocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "Allocation Details:")?;
        writeln!(f, "├── Size: {}", format_bytes(self.size))?;
        writeln!(f, "├── Peak Memory: {}", format_bytes(self.peak_mem))?;
        writeln!(f, "├── Peak Timestamps: {:?}", self.peak_timestamps)?;
        writeln!(
            f,
            "├── Timesteps: start {}, stop {}",
            self.timesteps.first().unwrap_or(&0),
            self.timesteps.last().unwrap_or(&0)
        )?;
        writeln!(f, "├── Offsets: omitted")?;
        // Or print offsets if desired:
        // writeln!(f, "├── Offsets: {:?}", self.offsets)?;

        writeln!(f, "└── Callstack:")?;
        if self.callstack.is_empty() {
            writeln!(f, "    └── (empty callstack)")?;
        } else {
            for (i, frame) in self.callstack.iter().enumerate() {
                let prefix = if i == self.callstack.len() - 1 {
                    "    └──"
                } else {
                    "    ├──"
                };
                writeln!(f, "{} ({}){}", prefix, i, frame)?;
            }
        }

        Ok(())
    }
}

impl Allocation {
    pub fn is_alive_in_interval(&self, start: u64, stop: u64) -> bool {
        self.is_alive_at(start) && self.is_alive_at(stop)
    }

    pub fn is_alive_at(&self, timestamp: u64) -> bool {
        self.timesteps[0] <= timestamp && timestamp <= *self.timesteps.last().unwrap()
    }

    pub fn start_end_time(&self) -> (u64, u64) {
        (self.timesteps[0], *self.timesteps.last().unwrap())
    }
}

// Intermediate struct to help parse the structure of allocations.json
#[derive(Deserialize)]
pub struct RawAllocationData {
    pub timesteps: Vec<u64>,
    pub offsets: Vec<u64>,
    pub size: u64,
}

// Intermediate struct to help parse the structure of elements.json
// Each element in elements.json contains a list of frames.
#[derive(Deserialize)]
pub struct ElementData {
    pub frames: Vec<Frame>,
}
