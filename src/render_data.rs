use crate::geometry::AllocationGeometry;
use log::info;
use nalgebra::Vector2;
use rand::Rng;
use three_d::{CpuMesh, Matrix, Srgba};

/// After transform, [-1,1] x [-1,1] stays in window, others are not displayed.
#[derive(Clone, Copy, Debug)]
pub struct Transform {
    pub scale: Vector2<f64>,
    pub translate: Vector2<f64>,
}

impl Transform {
    pub fn identity() -> Self {
        Transform {
            scale: Vector2::new(1.0, 1.0),
            translate: Vector2::new(0.0, 0.0),
        }
    }

    #[rustfmt::skip]
    pub fn to_mat4(self) -> three_d::Mat4 {
        // do not format
        three_d::Mat4::new(
            // column major
            self.scale.x as f32,0.0                ,0.0,self.translate.x as f32,
            0.0                ,self.scale.y as f32,0.0,self.translate.y as f32,
            0.0                ,0.0                ,1.0,0.0,
            0.0                ,0.0                ,0.0,1.0,
        )
        .transpose() // now row major
    }
}

pub struct RenderData {
    pub verts: Vec<three_d::Vector3<f64>>,
    pub colors: Vec<Srgba>,
}

impl RenderData {
    pub fn from_allocations<'a>(allocations: impl Iterator<Item = &'a AllocationGeometry>) -> Self {
        info!("Converting geometries to render-able mesh...");

        // pack a random color with each allocation
        let mut rng = rand::rng();
        let alloc_colors = allocations.map(|alloc| {
            let r = rng.random_range(0..=255);
            let g = rng.random_range(0..=255);
            let b = rng.random_range(0..=255);
            let color = Srgba::new(r, g, b, 30);

            (alloc, color)
        });

        // prepare containers for geometry
        let mut verts = Vec::new();
        let mut vert_colors = Vec::new();

        for (alloc, color) in alloc_colors {
            for ivert in 0..alloc.num_steps() - 1 {
                let this_time = alloc.timesteps[ivert];
                let next_time = alloc.timesteps[ivert + 1];
                let this_lo = alloc.offsets[ivert];
                let next_lo = alloc.offsets[ivert + 1];
                let this_hi = this_lo + alloc.size;
                let next_hi = next_lo + alloc.size;

                let left_bot = three_d::Vector3::new(this_time, this_lo, 0.0);
                let left_top = three_d::Vector3::new(this_time, this_hi, 0.0);
                let right_bot = three_d::Vector3::new(next_time, next_lo, 0.0);
                let right_top = three_d::Vector3::new(next_time, next_hi, 0.0);

                // Triangle 1
                verts.push(left_bot);
                verts.push(right_bot);
                verts.push(left_top);

                // Triangle 2
                verts.push(left_top);
                verts.push(right_bot);
                verts.push(right_top);

                // colors for all verts
                for _ in 0..6 {
                    vert_colors.push(color);
                }
            }
        }

        assert!(
            verts.len() % 3 == 0,
            "Require 3 verts per triangle, got {}",
            verts.len()
        );

        Self {
            verts,
            colors: vert_colors,
        }
    }

    pub fn to_cpu_mesh(&self) -> CpuMesh {
        CpuMesh {
            positions: three_d::Positions::F64(self.verts.clone()),
            colors: Some(self.colors.clone()),
            indices: three_d::Indices::None,
            normals: None,
            tangents: None,
            uvs: None,
        }
    }
}
