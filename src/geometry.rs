use log::info;

use crate::allocation::Allocation;

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

pub struct TraceGeometry {
    pub allocations: Vec<AllocationGeometry>,
    pub max_size: f64,
    pub max_time: f64,
}

impl TraceGeometry {
    pub fn from_allocations(allocations: &Vec<Allocation>, resolution: (u32, u32)) -> Self {
        info!("Transforming allocations memory snap to geometries...");
        let max_size = allocations
            .iter()
            .map(|a| *a.offsets.iter().max().unwrap() + a.size) // maximum offset + self size
            .max()
            .unwrap_or(0) as f64;

        let max_time = allocations
            .iter()
            .map(|a| *a.timesteps.last().unwrap())
            .max()
            .unwrap_or(0) as f64;

        let resolution_x = resolution.0 as f64;
        let resolution_y = resolution.1 as f64;

        let geometries = allocations
            .iter()
            .map(|alloc| AllocationGeometry {
                // normalized
                // normalize timesteps
                timesteps: alloc
                    .timesteps
                    .iter()
                    .map(|t| *t as f64 / max_time * resolution_x)
                    .collect(),
                // normalize offsets
                offsets: alloc
                    .offsets
                    .iter()
                    .map(|off| *off as f64 / max_size * resolution_y)
                    .collect(),
                // normalize size
                size: alloc.size as f64 / max_size * resolution_y,
            })
            .collect();

        Self {
            allocations: geometries,
            max_size,
            max_time,
        }
    }
}
