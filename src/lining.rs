use crate::geometry::AllocationGeometry;
use three_d::{ColorMaterial, Context, Gm, Line, Object, Srgba};

pub fn generate_lining_mesh<'a>(
    context: &Context,
    allocation: &'a AllocationGeometry,
) -> Vec<Box<dyn Object>> {
    let material = ColorMaterial {
        color: Srgba::WHITE, // colors are mixed (component-wise multiplied)
        ..Default::default()
    };

    let mut lines: Vec<Box<dyn Object>> = Vec::new();

    let left_bot = (allocation.timesteps[0] as f32, allocation.offsets[0] as f32);
    let left_top = (
        allocation.timesteps[0] as f32,
        allocation.offsets[0] as f32 + allocation.size as f32,
    );

    let left_line = Line::new(
        context, left_bot, left_top, 3.0, // hardcode for now
    );

    let right_bot = (
        *allocation.timesteps.last().unwrap() as f32,
        *allocation.offsets.last().unwrap() as f32,
    );
    let right_top = (
        *allocation.timesteps.last().unwrap() as f32,
        *allocation.offsets.last().unwrap() as f32 + allocation.size as f32,
    );

    let right_line = Line::new(
        context, right_bot, right_top, 3.0, // hardcode for now
    );

    lines.push(Box::new(Gm::new(left_line, material.clone())));
    lines.push(Box::new(Gm::new(right_line, material.clone())));

    for i in 0..allocation.num_steps() - 1 {
        let bot_end1 = (allocation.timesteps[i] as f32, allocation.offsets[i] as f32);
        let bot_end2 = (
            allocation.timesteps[i + 1] as f32,
            allocation.offsets[i + 1] as f32,
        );

        let top_end1 = (
            allocation.timesteps[i] as f32,
            allocation.offsets[i] as f32 + allocation.size as f32,
        );
        let top_end2 = (
            allocation.timesteps[i + 1] as f32,
            allocation.offsets[i + 1] as f32 + allocation.size as f32,
        );

        let bot_line = Line::new(
            context, bot_end1, bot_end2, 3.0, // hardcode for now
        );

        let top_line = Line::new(
            context, top_end1, top_end2, 3.0, // hardcode for now
        );

        // lines = lines
        //     .chain(&Gm::new(bot_line, material.clone()))
        //     .chain(&Gm::new(top_line, material.clone()));

        lines.push(Box::new(Gm::new(bot_line, material.clone())));
        lines.push(Box::new(Gm::new(top_line, material.clone())));
    }

    lines
}
