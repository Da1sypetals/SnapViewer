use nalgebra::Vector2;

/// After transform, [-1,1] x [-1,1] stays in window, others are not displayed.
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
}

pub type Color = nalgebra::SVector<f32, 4>;

pub struct RenderData {
    /// Verts: [x1, y1, x2, y2, ...]
    /// Every 6 verts elements (3 coords per vert, 3 verts per triangle) forms a triangle. No need for indices.
    pub verts: Vec<f64>,

    /// Length should be equal to verts.len() / 6
    pub colors: Vec<Color>,

    pub transform: Transform,
}
