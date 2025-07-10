use crate::allocation::Allocation;
use indicatif::ProgressIterator;
use log::info;
use nalgebra::Vector2;
use std::sync::Arc;

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
    pub raw_allocs: Arc<[Allocation]>,
    pub allocations: Vec<AllocationGeometry>,
    pub max_size: f64,
    pub max_time: f64,
    resolution: (u32, u32),
}

impl TraceGeometry {
    /// Executed at start
    pub fn from_allocations(allocations: Arc<[Allocation]>, resolution: (u32, u32)) -> Self {
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
            .progress()
            .collect();

        Self {
            raw_allocs: allocations,
            allocations: geometries,
            max_size,
            max_time,
            resolution,
        }
    }

    /// return index of allocation
    /// FIXME: this is a fucking naive implementation
    pub fn find_by_pos(&self, pos: Vector2<f32>) -> Option<usize> {
        let x = pos.x as f64; // time
        let y = pos.y as f64; // memory
        for (ialloc, alloc) in self.allocations.iter().enumerate() {
            // simple culling: check if x is in range of allocation, if not, position cannot be in this allocation
            if x < alloc.timesteps[0] || x > *alloc.timesteps.last().unwrap() {
                continue;
            }

            // find index of x in timesteps
            let idx = match alloc.timesteps.binary_search_by(|&e| e.total_cmp(&x)) {
                Ok(i) => i,
                Err(i) => i,
            };

            // find the interval index of x in timesteps
            let left_idx = idx - 1;
            let right_idx = idx;

            // get the time of the left and right interval
            let left_time = alloc.timesteps[left_idx];
            let right_time = alloc.timesteps[right_idx];

            let left_lo = alloc.offsets[left_idx];
            let right_lo = alloc.offsets[right_idx];
            let left_hi = alloc.offsets[left_idx] + alloc.size;
            let right_hi = alloc.offsets[right_idx] + alloc.size;

            // lerp ratio
            let t = (x - left_time) / (right_time - left_time);
            let lo = left_lo + (right_lo - left_lo) * t;
            let hi = left_hi + (right_hi - left_hi) * t;

            if lo <= y && y <= hi {
                info!("Find by pos: ok");
                return Some(ialloc);
            }
        }

        info!("Find by pos: failed");
        None
    }

    pub fn allocation_info(&self, idx: usize) -> String {
        self.raw_allocs[idx].to_string()
    }

    /// y_world: y position (world coords)
    /// Allow negative memory
    pub fn yworld2memory(&self, y_world: f32) -> i64 {
        (y_world as f64 * self.max_size / self.resolution.1 as f64) as i64
    }

    /// y_world: y position (world coords)
    /// Allow negative memory
    pub fn xworld2timestamp(&self, x_world: f32) -> i64 {
        (x_world as f64 * self.max_time / self.resolution.0 as f64) as i64
    }
}
