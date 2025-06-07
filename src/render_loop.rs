#![allow(warnings)]

use crate::{
    allocation::Allocation,
    geometry::{AllocationGeometry, TraceGeometry},
    load::{load_allocations, read_snap_from_zip},
    render_data::{RenderData, Transform},
    ticks::{self, TickGenerator},
    ui::{TranslateDir, WindowTransform},
    utils::{format_bytes, format_bytes_precision},
};
use log::info;
use nalgebra::Vector2;
use three_d::{
    Camera, Circle, ClearState, ColorMaterial, Event, FrameOutput, Geometry, Gm, Line, Mesh,
    MouseButton, Rectangle, Srgba, Viewport, Window, WindowSettings, context::SRGB8_ALPHA8,
    degrees, vec2, vec3,
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
        let elapsed = self.timer.elapsed().as_secs_f64();
        if elapsed >= 1.0 {
            info!("FPS: {:.2}", self.frame as f64 / elapsed as f64);
            self.timer = std::time::Instant::now();
            self.frame = 0;
        }
    }
}

pub struct DecayingWhiteColor {
    pub fade_time: f64,
    pub time: f64,
    pub material: ColorMaterial,
    pub target_color: Srgba,
}

impl DecayingWhiteColor {
    pub fn new(fade_time: f64, target_color: Srgba) -> Self {
        Self {
            fade_time,
            time: 0.0,
            material: ColorMaterial {
                color: Srgba::WHITE,
                ..Default::default()
            },
            target_color,
        }
    }

    pub fn tick(&mut self, dt: f64) {
        // at most fade_time seconds
        self.time = self.fade_time.min(self.time + dt);
        self.update_color();
    }

    pub fn reset(&mut self, target_color: Srgba) {
        self.time = 0.0;
        self.target_color = target_color;
        self.update_color();
    }

    pub fn update_color(&mut self) {
        // time = 0 -> alpha = 1.0
        // let mut color = Srgba::WHITE;
        // color.a = ((1.0 - self.time / self.fade_time) * 255.0) as u8;

        let t = 1.0 - self.time / self.fade_time;
        // lerp between
        let color = Srgba {
            r: self.target_color.r + ((255 - self.target_color.r) as f64 * t) as u8,
            g: self.target_color.g + ((255 - self.target_color.g) as f64 * t) as u8,
            b: self.target_color.b + ((255 - self.target_color.b) as f64 * t) as u8,
            a: 255,
        };
        self.material.color = color;
    }

    pub fn material(&self) -> ColorMaterial {
        self.material.clone()
    }
}

pub struct RenderLoop {
    pub trace_geom: TraceGeometry,
    pub resolution: (u32, u32),
    pub selected_mesh: Option<Gm<Mesh, ColorMaterial>>,
}

impl RenderLoop {
    pub fn from_allocations(allocations: Vec<Allocation>, resolution: (u32, u32)) -> Self {
        Self {
            trace_geom: TraceGeometry::from_allocations(allocations, resolution),
            resolution,
            selected_mesh: None,
        }
    }

    pub fn run(mut self) {
        let window = Window::new(WindowSettings {
            title: "SnapViewer".to_string(),
            min_size: self.resolution,
            max_size: Some(self.resolution),
            ..Default::default()
        })
        .unwrap();
        let context = window.gl();
        let scale_factor = window.device_pixel_ratio();
        let (width, height) = window.size();

        let rdata = RenderData::from_allocations(self.trace_geom.allocations.iter());

        let cpumesh = rdata.to_cpu_mesh();
        let mut mesh = Gm::new(
            Mesh::new(&context, &cpumesh),
            ColorMaterial {
                color: Srgba::WHITE, // colors are mixed (component-wise multiplied)
                ..Default::default()
            },
        );

        let transform = Transform::identity();
        let mut win_trans = WindowTransform::new(self.resolution);
        win_trans.set_zoom_limits(0.75, self.trace_geom.max_time as f32 / 100.0);

        let tickgen = TickGenerator::jbmono(self.resolution, 20.0);

        // start a timer
        let mut timer = FpsTimer::new();
        let mut decaying_white = DecayingWhiteColor::new(0.8, Srgba::WHITE);

        window.render_loop(move |mut frame_input| {
            let mut mesh_iter = std::iter::once(&mesh);

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

                                // try to find allocation by cursor position
                                let alloc_idx = self.trace_geom.find_by_pos(cursor_world_pos);
                                info!("Find by pos results: alloc id: {:?}", alloc_idx);

                                // if we found an allocation at cursor position
                                if let Some(idx) = alloc_idx {
                                    // print allocation info
                                    println!(
                                        "Allocation #{}\n{}",
                                        idx,
                                        self.trace_geom.allocation_info(idx)
                                    );

                                    // animate allocated mesh
                                    let alloc_rdata = RenderData::from_allocations_with_z(
                                        std::iter::once((
                                            &self.trace_geom.allocations[idx],
                                            Srgba::WHITE,
                                        )),
                                        0.005,
                                    );
                                    let alloc_mesh = Gm::new(
                                        Mesh::new(&context, &alloc_rdata.to_cpu_mesh()),
                                        decaying_white.material(),
                                    );
                                    self.selected_mesh = Some(alloc_mesh);

                                    // The original color of the allocation
                                    let original_color = rdata.alloc_colors[idx];
                                    decaying_white.reset(original_color);
                                }
                            }
                            MouseButton::Right => {
                                let cursor_world_pos = win_trans.screen2world(position.into());
                                info!(
                                    "Right click world pos: ({}, {})",
                                    cursor_world_pos.x, cursor_world_pos.y
                                );

                                // print memory position at cursor
                                println!(
                                    "Cursor is at memory: {}",
                                    format_bytes_precision(
                                        self.trace_geom.world2memory(cursor_world_pos.y),
                                        3
                                    )
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
                            win_trans.zoom_in(position.into());
                        } else if delta.1 < 0.0 {
                            win_trans.zoom_out(position.into());
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
                                info!("{:?},", key);
                            }
                        }
                    }
                    _ => {}
                }
            }
            let cam = win_trans.camera(frame_input.viewport);

            mesh.set_transformation(transform.to_mat4());

            let high_bytes = self.trace_geom.world2memory(win_trans.ytop_world());
            let low_bytes = self.trace_geom.world2memory(win_trans.ybot_world());
            let ticks = tickgen.generate_memory_ticks(
                low_bytes,
                high_bytes,
                win_trans.scale(),
                win_trans.center,
                &context,
            );

            let mut allocation_meshes = vec![&mesh];
            if let Some(selected_mesh) = &mut self.selected_mesh {
                selected_mesh.material = decaying_white.material();
                allocation_meshes.push(selected_mesh);
            }

            frame_input
                .screen()
                .clear(ClearState::color_and_depth(1.0, 1.0, 1.0, 1.0, 1.0))
                .render(
                    cam,
                    // line.into_iter()
                    //     .chain(&rectangle)
                    //     .chain(&circle)
                    //     .chain(&mesh),
                    ticks.iter().chain(allocation_meshes.into_iter()),
                    &[],
                );

            timer.tick();
            decaying_white.tick(frame_input.elapsed_time / 1000.0); // this is MS

            FrameOutput::default()
        });
    }
}
