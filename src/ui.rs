use nalgebra::Vector2;
use three_d::{vec3, Camera, Viewport};

#[derive(Debug)]
pub struct WindowTransform {
    pub translate: Vector2<f32>,
    pub zoom: f32,

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
