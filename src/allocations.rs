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

    /// Verts: [x1, y1, z1, x2, y2, z2, ...]
    /// Every 9 verts elements (3 coords per vert, 3 verts per triangle) forms a triangle.
    pub fn append_triangles(&self, verts: &mut Vec<f64>) {
        for i in 0..self.num_steps() - 1 {
            let this_time = self.timesteps[i];
            let next_time = self.timesteps[i + 1];
            let this_lo = self.offsets[i];
            let next_lo = self.offsets[i + 1];
            let this_hi = this_lo + self.size;
            let next_hi = next_lo + self.size;

            // tri 1
            // bottom-left
            verts.push(this_time);
            verts.push(this_lo);
            // bottom-right
            verts.push(next_time);
            verts.push(next_lo);
            // top-left
            verts.push(this_time);
            verts.push(this_hi);

            // tri 2
            // top-left
            verts.push(this_time);
            verts.push(this_hi);
            // bottom-right
            verts.push(next_time);
            verts.push(next_lo);
            // top-right
            verts.push(next_time);
            verts.push(next_hi);
        }
    }
}
