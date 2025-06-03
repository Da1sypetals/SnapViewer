use crate::utils::{format_bytes, format_bytes_precision, generate_ticks};
use nalgebra::Vector2;
use three_d::{
    ColorMaterial, Context, Gm, Mat4, Mesh, Srgba, TextGenerator, TextLayoutOptions, Vector3,
};

pub struct TickGenerator<'a> {
    pub generator: TextGenerator<'a>,
    pub resolution: (u32, u32),
    pub fontsize_px: f32,
}

impl<'a> TickGenerator<'a> {
    pub fn jbmono(resolution: (u32, u32), fontsize_px: f32) -> Self {
        let generator = TextGenerator::new(
            include_bytes!("../../assets/JetBrainsMono-Medium.ttf"),
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
                let text = format!("—— {}", format_bytes_precision(bytes as i64, 4));
                self.generate_text_mesh(&text, y_ratio, scale, screen_center_world, context)
            })
            .collect::<Vec<_>>()
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

        let transform =
            Mat4::from_translation(Vector3::new(font_pos_world.x, font_pos_world.y, 0.0))
                * Mat4::from_scale(scale);

        gm.set_transformation(transform);

        gm
    }
}
