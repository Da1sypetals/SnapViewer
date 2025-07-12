use crate::utils::format_bytes;
use serde::Deserialize;
use std::fmt::{Display, Formatter, Result};

// Corresponds to the Python Allocation dataclass
#[derive(Deserialize, Debug, Clone)]
pub struct Allocation {
    pub timesteps: Vec<u64>, // x coords, sorted
    pub offsets: Vec<u64>,   // y coords, length same as `timesteps`
    pub size: u64,           // height (sweep distance)
    pub peak_mem: u64,
    pub peak_timestamps: Vec<u64>, // reaches its peak at these timestamps
}

impl Display for Allocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "Allocation Details:")?;

        // Callstack first

        // Other details in their original order
        writeln!(f, "|- Size: {}", format_bytes(self.size as i64))?;
        writeln!(f, "|- Peak Memory: {}", format_bytes(self.peak_mem as i64))?;
        writeln!(f, "|- Peak Timestamps: {:?}", self.peak_timestamps)?;
        writeln!(
            f,
            "|- Timesteps: start {}, stop {}",
            self.timesteps.first().unwrap_or(&0),
            self.timesteps.last().unwrap_or(&0)
        )?;
        writeln!(f, "|- Offsets: omitted")?;
        // Or print offsets if desired:
        // writeln!(f, "└── Offsets: {:?}", self.offsets)?;

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
