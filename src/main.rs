#![allow(warnings)]

use log::info;
use nalgebra::Vector2;
use snapviewer::{
    geometry::{AllocationGeometry, TraceGeometry},
    load::{load_allocations, read_snap_from_jsons, read_snap_from_zip},
    render_data::{RenderData, Transform},
};
use three_d::{
    degrees, vec2, Camera, Circle, ClearState, ColorMaterial, Event, FrameOutput, Geometry, Gm,
    Line, Mesh, MouseButton, Rectangle, Srgba, Window, WindowSettings,
};

pub fn load_geom(resolution: (u32, u32)) -> RenderData {
    info!("Reading snapshot from disk...");
    // let rawsnap = read_snap_from_zip("snap/transformer.zip").unwrap();
    let rawsnap = read_snap_from_jsons(
        "/home/da1sypetals/dev/torch-snapshot/snapshots/allocations.json",
        "/home/da1sypetals/dev/torch-snapshot/snapshots/elements.json",
    )
    .unwrap();

    info!("Loading allocations from zip...");
    let allocs = load_allocations(rawsnap).unwrap();

    let tracegeom = TraceGeometry::from_allocations(&allocs, resolution);

    RenderData::from_allocations(tracegeom.allocations)
}

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

pub fn main() {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(log::LevelFilter::Off)
        .filter_module("snapviewer", log::LevelFilter::Info)
        .init();

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

    let rdata = load_geom((resolution.0, (resolution.1 as f32 * 0.9) as u32));
    let cpumesh = rdata.to_cpu_mesh();
    let mut mesh = Gm::new(
        Mesh::new(&context, &cpumesh),
        ColorMaterial {
            color: Srgba::WHITE, // colors are mixed (component-wise multiplied)
            ..Default::default()
        },
    );

    let transform = Transform {
        scale: Vector2::new(0.9, 0.9),
        translate: Vector2::new(50., 50.),
    };

    // start a timer
    let mut timer = FpsTimer::new();

    window.render_loop(move |frame_input| {
        for event in frame_input.events.iter() {
            if let Event::MousePress {
                button,
                position,
                modifiers,
                ..
            } = *event
            {}
        }

        mesh.set_transformation(transform.to_mat4());

        frame_input
            .screen()
            .clear(ClearState::color_and_depth(1.0, 1.0, 1.0, 1.0, 1.0))
            .render(
                Camera::new_2d(frame_input.viewport),
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
