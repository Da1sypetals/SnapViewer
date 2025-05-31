use nalgebra::Vector2;
use three_d::{vec3, Camera, Viewport};

#[derive(Debug)]
pub struct WindowTransform {
    pub translate: Vector2<f32>,
    pub zoom: f32,

    resolution: (u32, u32),

    max_zoom: f32,
    translate_min: Vector2<f32>,
    translate_max: Vector2<f32>,

    zoom_step: f32,
    translate_step: f32,
}

impl WindowTransform {
    pub fn new(resolution: (u32, u32)) -> Self {
        Self {
            translate: Vector2::zeros(),
            zoom: 1.0,
            resolution,
            max_zoom: 24.0,
            translate_max: Vector2::new(resolution.0 as f32 * 0.5, resolution.1 as f32 * 0.5),
            translate_min: Vector2::new(resolution.0 as f32 * (-0.5), resolution.1 as f32 * (-0.5)),
            zoom_step: 0.25,     // times
            translate_step: 5.0, // pixels
        }
    }

    pub fn scale(&self) -> f32 {
        self.zoom.recip()
    }

    pub fn camera(&self, viewport: Viewport) -> Camera {
        let center_x = viewport.width as f32 * 0.5 + self.translate.x;
        let center_y = viewport.height as f32 * 0.5 + self.translate.y;

        Camera::new_orthographic(
            viewport,
            vec3(center_x, center_y, 1.0),
            vec3(center_x, center_y, 0.0),
            vec3(0.0, 1.0, 0.0),
            // in real world units, NOT PIXELS
            viewport.height as f32 * self.scale(),
            0.0,
            10.0,
        )
    }

    pub fn screen2world(&self, screen_pos: (u32, u32)) -> Vector2<f32> {
        let screen_pos = (screen_pos.0 as i32, screen_pos.1 as i32);
        let center = (
            (self.resolution.0 / 2) as i32,
            (self.resolution.1 / 2) as i32,
        );
        let rel_center = (screen_pos.0 - center.0, screen_pos.1 - center.1);
        let scale = self.scale();
        let rel_world = Vector2::new(rel_center.0 as f32 * scale, rel_center.1 as f32 * scale);

        self.translate + rel_world
    }
}

pub enum TranslateDir {
    Left,
    Right,
    Up,
    Down,
}

impl WindowTransform {
    // actions
    pub fn enforce_boundaries(&mut self) {
        self.translate.x = self.translate.x.max(self.translate_min.x);
        self.translate.x = self.translate.x.min(self.translate_max.x);
        self.translate.y = self.translate.y.max(self.translate_min.y);
        self.translate.y = self.translate.y.min(self.translate_max.y);
    }

    pub fn zoom_in(&mut self) {
        self.zoom = self.max_zoom.min(self.zoom + self.zoom_step);
    }

    pub fn zoom_out(&mut self) {
        self.zoom = 1.0f32.max(self.zoom - self.zoom_step);
    }

    pub fn translate(&mut self, dir: TranslateDir) {
        match dir {
            TranslateDir::Left => self.translate.x -= self.translate_step,
            TranslateDir::Right => self.translate.x += self.translate_step,
            TranslateDir::Up => self.translate.y += self.translate_step,
            TranslateDir::Down => self.translate.y -= self.translate_step,
        }

        self.enforce_boundaries();
    }
}
