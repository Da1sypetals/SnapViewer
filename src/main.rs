#![allow(warnings)]

use log::info;
use snapviewer::{
    geometry::{AllocationGeometry, TraceGeometry},
    load::{load_allocations, read_snap_from_jsons, read_snap_from_zip},
    render_data::RenderData,
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

pub fn main() {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(log::LevelFilter::Off)
        .filter_module("snapviewer", log::LevelFilter::Info)
        .init();

    let resolution = (1280, 720);

    let window = Window::new(WindowSettings {
        title: "Tomi Viewer".to_string(),
        max_size: Some(resolution),
        ..Default::default()
    })
    .unwrap();
    let context = window.gl();
    let scale_factor = window.device_pixel_ratio();
    let (width, height) = window.size();

    let rdata = load_geom(resolution);
    let cpumesh = rdata.to_cpu_mesh();
    let mut mesh = Gm::new(
        Mesh::new(&context, &cpumesh),
        ColorMaterial {
            color: Srgba::WHITE, // colors are mixed (component-wise multiplied)
            ..Default::default()
        },
    );

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

        println!("render!");

        FrameOutput::default()
    });
}
