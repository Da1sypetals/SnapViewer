use crate::geometry::AllocationGeometry;
use indicatif::ProgressIterator;
use log::info;
use rand::Rng;
use three_d::{CpuMesh, Srgba};

pub fn from_allocations<'a>(
    allocations: impl ExactSizeIterator<Item = &'a AllocationGeometry>, // required for progress bar
) -> (CpuMesh, Vec<Srgba>) {
    info!("Converting geometries to render-able mesh...");

    // pack a random color with each allocation
    let mut rng = rand::rng();
    let alloc_colors = allocations
        .map(|alloc| {
            let color = loop {
                let r: u32 = rng.random_range(0..=255);
                let g: u32 = rng.random_range(0..=255);
                let b: u32 = rng.random_range(0..=255);

                // Reject colors that are too light or too dark
                if 150 < r + g + b && r + g + b < 600 {
                    break Srgba::new(r as u8, g as u8, b as u8, 30);
                }
            };

            (alloc, color)
        })
        .progress();

    from_allocations_with_z(alloc_colors, 0.0)
}

pub fn from_allocations_with_z<'a>(
    alloc_zip_colors: impl Iterator<Item = (&'a AllocationGeometry, Srgba)>,
    z: f64,
) -> (CpuMesh, Vec<Srgba>) {
    // prepare containers for geometry
    let mut verts = Vec::new();
    let mut vert_colors = Vec::new();
    let mut alloc_colors = Vec::new();

    for (alloc, color) in alloc_zip_colors {
        alloc_colors.push(color);
        for ivert in 0..alloc.num_steps() - 1 {
            let this_time = alloc.timesteps[ivert];
            let next_time = alloc.timesteps[ivert + 1];
            let this_lo = alloc.offsets[ivert];
            let next_lo = alloc.offsets[ivert + 1];
            let this_hi = this_lo + alloc.size;
            let next_hi = next_lo + alloc.size;

            // vertices that make up the quad
            let left_bot = three_d::Vector3::new(this_time, this_lo, z);
            let left_top = three_d::Vector3::new(this_time, this_hi, z);
            let right_bot = three_d::Vector3::new(next_time, next_lo, z);
            let right_top = three_d::Vector3::new(next_time, next_hi, z);

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

    (
        CpuMesh {
            positions: three_d::Positions::F64(verts),
            colors: Some(vert_colors),
            indices: three_d::Indices::None,
            normals: None,
            tangents: None,
            uvs: None,
        },
        alloc_colors,
    )
}
