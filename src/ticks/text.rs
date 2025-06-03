use nalgebra::Vector2;
use three_d::{
    ColorMaterial, Context, Gm, Mat4, Mesh, Srgba, TextGenerator, TextLayoutOptions, Vector3,
};

pub struct TickText<'a> {
    pub generator: TextGenerator<'a>,
    pub resolution: (u32, u32),
    pub fontsize_px: f32,
    pub context: &'a Context,
}

impl<'a> TickText<'a> {
    pub fn jbmono(resolution: (u32, u32), fontsize_px: f32, context: &'a Context) -> Self {
        let generator = TextGenerator::new(
            include_bytes!("../../assets/JetBrainsMono-Medium.ttf"),
            0,
            30.0,
        )
        .unwrap();

        Self {
            generator,
            resolution,
            fontsize_px,
            context,
        }
    }

    /// y is 0~1, 0 for left-bottom, 1 for left-top.
    /// scale: reciprocal of zoom
    pub fn generate_text_mesh(
        &self,
        text: &str,
        y_ratio: f32,
        scale: f32,
        screen_center_world: Vector2<f32>,
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
            Mesh::new(self.context, &cpumesh),
            ColorMaterial {
                color: Srgba::BLACK,
                ..Default::default()
            },
        );

        gm.set_transformation(Mat4::from_translation(Vector3::new(
            font_pos_world.x,
            font_pos_world.y,
            0.0,
        )));

        gm
    }
}
