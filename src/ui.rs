use nalgebra::Vector2;
use three_d::{vec3, Camera, Viewport};

#[derive(Debug)]
pub struct WindowTransform {
    pub translate: Vector2<f32>,
    pub zoom: f32,

    resolution: (u32, u32),

    min_zoom: f32,
    max_zoom: f32,
    translate_min: Vector2<f32>,
    translate_max: Vector2<f32>,

    zoom_step: f32,
    translate_step_multiplier: f32,
}

impl WindowTransform {
    pub fn new(resolution: (u32, u32)) -> Self {
        Self {
            translate: Vector2::zeros(),
            zoom: 1.0,
            resolution,
            min_zoom: 0.75,
            max_zoom: 36.0, // TODO: update to max_timesteps / 100
            translate_max: Vector2::new(resolution.0 as f32 * 0.5, resolution.1 as f32 * 0.5),
            translate_min: Vector2::new(resolution.0 as f32 * (-0.5), resolution.1 as f32 * (-0.5)),
            zoom_step: 0.16, // everytime * (1.0 + zoom_step)
            translate_step_multiplier: 24.0,
        }
    }

    pub fn scale(&self) -> f32 {
        self.zoom.recip()
    }

    pub fn translate_step(&self) -> f32 {
        self.translate_step_multiplier / self.zoom.sqrt()
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

    pub fn screen2world(&self, screen_pos: (f32, f32)) -> Vector2<f32> {
        let center = Vector2::new(
            (self.resolution.0 / 2) as f32,
            (self.resolution.1 / 2) as f32,
        );
        let rel_center: Vector2<f32> = Vector2::new(screen_pos.0, screen_pos.1) - center;
        let scale = self.scale();
        let rel_world = rel_center * scale;

        center
        + self.translate // center translate relative to  (w/2, h/2)
        + rel_world
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

    pub fn update_zoom(&mut self, new_zoom: f32, screen_pos: (f32, f32)) {
        let cursor_world_pos = self.screen2world(screen_pos);
        let cursor_to_center = self.translate
            + Vector2::new(
                self.resolution.0 as f32 / 2.0,
                self.resolution.1 as f32 / 2.0,
            )
            - cursor_world_pos;

        let prev_zoom = self.zoom;
        self.zoom = new_zoom;

        let new_center = cursor_world_pos + (self.zoom / prev_zoom).recip() * cursor_to_center
            - Vector2::new(
                self.resolution.0 as f32 / 2.0,
                self.resolution.1 as f32 / 2.0,
            );

        self.translate = new_center;
    }

    pub fn zoom_in(&mut self, screen_pos: (f32, f32)) {
        self.update_zoom(
            self.max_zoom.min(self.zoom * (1.0 + self.zoom_step)),
            screen_pos,
        );
    }

    pub fn zoom_out(&mut self, screen_pos: (f32, f32)) {
        self.update_zoom(
            self.min_zoom.max(self.zoom * (1.0 - self.zoom_step)),
            screen_pos,
        );
    }

    pub fn translate(&mut self, dir: TranslateDir) {
        match dir {
            TranslateDir::Left => self.translate.x -= self.translate_step(),
            TranslateDir::Right => self.translate.x += self.translate_step(),
            TranslateDir::Up => self.translate.y += self.translate_step(),
            TranslateDir::Down => self.translate.y -= self.translate_step(),
        }

        self.enforce_boundaries();
    }
}
