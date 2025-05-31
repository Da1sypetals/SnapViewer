use three_d::{Srgba, Vector3};

/// 64 MB should be large enough to distinguish
const MEM_SCALE: f64 = (1024 * 1024 * 64) as f64;
const TIME_SCALE: f64 = 100_000 as f64;

pub struct Allocation {
    pub timesteps: Vec<u64>,
    pub offsets: Vec<u64>,
    pub size: u64,
}

impl Allocation {
    pub fn to_geometry(self) -> AllocationGeometry {
        let timesteps = self
            .timesteps
            .into_iter()
            .map(|t| t as f64 / TIME_SCALE)
            .collect();

        let offsets = self
            .offsets
            .into_iter()
            .map(|off| off as f64 / MEM_SCALE)
            .collect();

        AllocationGeometry {
            timesteps,
            offsets,
            size: self.size as f64 / MEM_SCALE,
        }
    }
}

pub struct AllocationGeometry {
    pub timesteps: Vec<f64>,
    pub offsets: Vec<f64>,
    pub size: f64,
}

impl AllocationGeometry {
    pub fn num_steps(&self) -> usize {
        debug_assert_eq!(
            self.timesteps.len(),
            self.offsets.len(),
            "timesteps and offsets must have same length, got timesteps: {}, offsets: {}",
            self.timesteps.len(),
            self.offsets.len()
        );

        self.timesteps.len()
    }

    
}
