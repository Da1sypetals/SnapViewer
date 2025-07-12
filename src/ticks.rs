use crate::{constants::INTERVALS, utils::format_bytes_precision};
use nalgebra::Vector2;
use three_d::{
    ColorMaterial, Context, Gm, Mat4, Mesh, Srgba, TextGenerator, TextLayoutOptions, Vector3,
};

pub const TICKS_FLOAT_Z: f32 = 0.01;

pub struct TickGenerator<'a> {
    pub generator: TextGenerator<'a>,
    pub resolution: (u32, u32),
    pub fontsize_px: f32,
}

impl<'a> TickGenerator<'a> {
    pub fn jbmono(resolution: (u32, u32), fontsize_px: f32) -> Self {
        let generator = TextGenerator::new(
            include_bytes!("../assets/JetBrainsMono-Medium.ttf"),
            0,
            fontsize_px,
        )
        .unwrap();

        Self {
            generator,
            resolution,
            fontsize_px,
        }
    }

    pub fn generate_memory_ticks(
        &self,
        low_bytes: i64,
        high_bytes: i64,
        scale: f32,
        screen_center_world: Vector2<f32>, // world coords of the screen center
        context: &'a Context,
    ) -> Vec<Gm<Mesh, ColorMaterial>> {
        // 1. generate ticks as a list of u64
        let ticks_bytes = generate_ticks(low_bytes, high_bytes);

        // 2. map ticks to meshes
        ticks_bytes
            .into_iter()
            .map(|bytes| {
                let y_ratio = (bytes - low_bytes) as f32 / (high_bytes - low_bytes) as f32;
                let text = format!("—— {}", format_bytes_precision(bytes, 4));
                self.generate_text_mesh(&text, y_ratio, scale, screen_center_world, context)
            })
            .collect()
    }
}

impl<'a> TickGenerator<'a> {
    /// y is 0~1, 0 for left-bottom, 1 for left-top.
    /// scale: reciprocal of zoom
    pub fn generate_text_mesh(
        &self,
        text: &str,
        y_ratio: f32,
        scale: f32,
        screen_center_world: Vector2<f32>, // world coords of the screen center
        context: &'a Context,
    ) -> Gm<Mesh, ColorMaterial> {
        let screen_pos_y_px = y_ratio * self.resolution.1 as f32 - self.fontsize_px / 2.0; // align font height center

        let center2pos_world = scale
            * Vector2::new(
                -((self.resolution.0 / 2) as f32),
                screen_pos_y_px - (self.resolution.1 / 2) as f32,
            );

        let font_pos_world = screen_center_world + center2pos_world;
        let cpumesh = self.generator.generate(text, TextLayoutOptions::default());

        let mut gm = Gm::new(
            Mesh::new(context, &cpumesh),
            ColorMaterial {
                color: Srgba::BLACK,
                ..Default::default()
            },
        );

        let scale_transform = Mat4::from_scale(scale);
        let translate_transform = Mat4::from_translation(Vector3::new(
            font_pos_world.x,
            font_pos_world.y,
            TICKS_FLOAT_Z,
        ));

        // first scale, then translate
        let transform = translate_transform * scale_transform;

        gm.set_transformation(transform);

        gm
    }
}

fn choose_interval(a: f64, b: f64, min_ticks: usize) -> f64 {
    let span = (b - a).abs();
    if span == 0.0 {
        return 1.0;
    }
    let valid_intervals: Vec<f64> = INTERVALS
        .iter()
        .filter_map(|&i| {
            if (span / i) > min_ticks as f64 {
                Some(i)
            } else {
                None
            }
        })
        .collect();

    if valid_intervals.is_empty() {
        // In Python, min(intervals) would be 4^0 = 1.0
        return INTERVALS.into_iter().next().unwrap_or(1.0);
    }
    *valid_intervals
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(&1.0)
}

fn generate_ticks_f64(a: f64, b: f64, interval: f64) -> Vec<f64> {
    let min_val = a.min(b);
    let max_val = a.max(b);
    let mut ticks = Vec::new();
    let mut i = 0;
    loop {
        let tick = i as f64 * interval;
        // Adding a small epsilon for floating point comparison robustness
        if tick > max_val + f64::EPSILON {
            break;
        }
        if tick >= min_val - f64::EPSILON {
            ticks.push(tick);
        }
        i += 1;
    }
    ticks
}

pub fn generate_ticks(low_bytes: i64, high_bytes: i64) -> Vec<i64> {
    let a = low_bytes as f64;
    let b = high_bytes as f64;
    let min_ticks = 8; // Default value from the Python function

    let interval = choose_interval(a, b, min_ticks);
    let ticks_f64 = generate_ticks_f64(a, b, interval);

    ticks_f64.into_iter().map(|t| t as i64).collect()
}

#[cfg(test)]
mod tests {
    use crate::ticks::generate_ticks;

    #[test]
    fn test_ticks() {
        let ticks = generate_ticks(1244, 23509823);
        assert_eq!(
            ticks,
            vec![
                1048576, 2097152, 3145728, 4194304, 5242880, 6291456, 7340032, 8388608, 9437184,
                10485760, 11534336, 12582912, 13631488, 14680064, 15728640, 16777216, 17825792,
                18874368, 19922944, 20971520, 22020096, 23068672
            ]
        );

        let ticks = generate_ticks(121244, 239823);
        assert_eq!(
            ticks,
            vec![
                122880, 126976, 131072, 135168, 139264, 143360, 147456, 151552, 155648, 159744,
                163840, 167936, 172032, 176128, 180224, 184320, 188416, 192512, 196608, 200704,
                204800, 208896, 212992, 217088, 221184, 225280, 229376, 233472, 237568
            ]
        );
    }
}
