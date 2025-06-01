#![allow(warnings)]

use crate::{
    allocation::Allocation,
    geometry::{AllocationGeometry, TraceGeometry},
    load::{load_allocations, read_snap_from_jsons, read_snap_from_zip, SnapType},
    render_data::{RenderData, Transform},
    ui::{TranslateDir, WindowTransform},
};
use log::info;
use nalgebra::Vector2;
use three_d::{
    degrees, vec2, vec3, Camera, Circle, ClearState, ColorMaterial, Event, FrameOutput, Geometry,
    Gm, Line, Mesh, MouseButton, Rectangle, Srgba, Viewport, Window, WindowSettings,
};

pub struct FpsTimer {
    pub timer: std::time::Instant,
    pub frame: u64,
}

impl FpsTimer {
    pub fn new() -> Self {
        Self {
            timer: std::time::Instant::now(),
            frame: 0,
        }
    }
    pub fn tick(&mut self) {
        self.frame += 1;
        if self.timer.elapsed().as_secs() > 1 {
            info!("FPS: {}", self.frame);
            self.timer = std::time::Instant::now();
            self.frame = 0;
        }
    }
}

pub struct RenderLoop {
    pub trace_geom: TraceGeometry,
    pub resolution: (u32, u32),
}

impl RenderLoop {
    pub fn from_allocations(allocations: Vec<Allocation>, resolution: (u32, u32)) -> Self {
        Self {
            trace_geom: TraceGeometry::from_allocations(&allocations, resolution),
            resolution,
        }
    }

    pub fn run(self) {
        let resolution = (2400, 1080);

        let window = Window::new(WindowSettings {
            title: "Tomi Viewer".to_string(),
            max_size: Some(resolution),
            ..Default::default()
        })
        .unwrap();
        let context = window.gl();
        let scale_factor = window.device_pixel_ratio();
        let (width, height) = window.size();

        let rdata = RenderData::from_allocations(&self.trace_geom.allocations);

        let cpumesh = rdata.to_cpu_mesh();
        let mut mesh = Gm::new(
            Mesh::new(&context, &cpumesh),
            ColorMaterial {
                color: Srgba::WHITE, // colors are mixed (component-wise multiplied)
                ..Default::default()
            },
        );

        let transform = Transform::identity();
        let mut win_trans = WindowTransform::new(resolution);
        // start a timer
        let mut timer = FpsTimer::new();

        window.render_loop(move |mut frame_input| {
            for event in frame_input.events.iter() {
                match *event {
                    Event::MousePress {
                        button,
                        position,
                        modifiers,
                        handled,
                    } => {
                        // rustfmt don't eliminate by brace
                        match button {
                            MouseButton::Left => {
                                let cursor_world_pos = win_trans.screen2world(position.into());
                                info!(
                                    "Left click world pos: ({}, {})",
                                    cursor_world_pos.x, cursor_world_pos.y
                                );

                                let alloc = self.trace_geom.find_by_pos(cursor_world_pos);
                                info!("Find by pos results: alloc id: {:?}", alloc);
                            }
                            MouseButton::Right => {
                                let cursor_world_pos = win_trans.screen2world(position.into());
                                info!(
                                    "Right click world pos: ({}, {})",
                                    cursor_world_pos.x, cursor_world_pos.y
                                );
                            }
                            MouseButton::Middle => {}
                        }
                    }
                    Event::MouseWheel {
                        delta,
                        position,
                        modifiers,
                        handled,
                    } => {
                        if delta.1 > 0.0 {
                            win_trans.zoom_in();
                        } else if delta.1 < 0.0 {
                            win_trans.zoom_out();
                        }
                    }
                    Event::KeyPress {
                        kind,
                        modifiers,
                        handled,
                    } => {
                        // placeholder
                        match kind {
                            three_d::Key::W => win_trans.translate(TranslateDir::Up),
                            three_d::Key::A => win_trans.translate(TranslateDir::Left),
                            three_d::Key::S => win_trans.translate(TranslateDir::Down),
                            three_d::Key::D => win_trans.translate(TranslateDir::Right),
                            key => {
                                dbg!(key);
                            }
                        }
                    }
                    _ => {}
                }
            }
            let cam = win_trans.camera(frame_input.viewport);

            mesh.set_transformation(transform.to_mat4());

            frame_input
                .screen()
                .clear(ClearState::color_and_depth(1.0, 1.0, 1.0, 1.0, 1.0))
                .render(
                    cam,
                    // line.into_iter()
                    //     .chain(&rectangle)
                    //     .chain(&circle)
                    //     .chain(&mesh),
                    mesh.into_iter(),
                    &[],
                );

            timer.tick();

            FrameOutput::default()
        });
    }
}
