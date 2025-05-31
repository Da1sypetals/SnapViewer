#![allow(warnings)]

use snapviewer::{allocations::AllocationGeometry, render_data::RenderData};
use three_d::{
    degrees, vec2, Camera, Circle, ClearState, ColorMaterial, Event, FrameOutput, Geometry, Gm,
    Line, Mesh, MouseButton, Rectangle, Srgba, Window, WindowSettings,
};

pub fn make_geom() -> RenderData {
    let g1 = AllocationGeometry {
        timesteps: vec![0.0, 50.0, 100.0, 150.0, 200.0],
        offsets: vec![0.0, 200.0, 100.0, 300.0, 50.0],
        size: 150.0,
    };

    let g2 = AllocationGeometry {
        timesteps: vec![0.0, 50.0, 100.0, 150.0, 200.0],
        offsets: vec![500.0, 570.0, 520.0, 550.0, 500.0],
        size: 80.0,
    };

    let g3 = AllocationGeometry {
        timesteps: vec![200.0, 300.0, 600.0, 800.0, 1100.0],
        offsets: vec![200.0, 270.0, 220.0, 250.0, 200.0],
        size: 200.0,
    };

    // let colors = vec![
    //     Srgba::new(50, 50, 200, 150),
    //     Srgba::new(200, 50, 50, 150),
    //     Srgba::new(50, 200, 50, 100),
    // ];

    // RenderData::with_colors(vec![g1, g2, g3], colors)
    RenderData::from_allocations(vec![g1, g2, g3])
}

pub fn main() {
    let window = Window::new(WindowSettings {
        title: "Tomi Viewer".to_string(),
        max_size: Some((1280, 720)),
        ..Default::default()
    })
    .unwrap();
    let context = window.gl();
    let scale_factor = window.device_pixel_ratio();
    let (width, height) = window.size();

    let rdata = make_geom();
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
            .clear(ClearState::color_and_depth(0.8, 0.8, 0.8, 1.0, 1.0))
            .render(
                Camera::new_2d(frame_input.viewport),
                // line.into_iter()
                //     .chain(&rectangle)
                //     .chain(&circle)
                //     .chain(&mesh),
                mesh.into_iter(),
                &[],
            );

        FrameOutput::default()
    });
}

// let mut rectangle = Gm::new(
//     Rectangle::new(
//         &context,
//         vec2(200.0, 200.0) * scale_factor,
//         degrees(45.0),
//         100.0 * scale_factor,
//         200.0 * scale_factor,
//     ),
//     ColorMaterial {
//         color: Srgba::RED,
//         ..Default::default()
//     },
// );
// let mut circle = Gm::new(
//     Circle::new(
//         &context,
//         vec2(500.0, 500.0) * scale_factor,
//         200.0 * scale_factor,
//     ),
//     ColorMaterial {
//         color: Srgba::BLUE,
//         ..Default::default()
//     },
// );
// let mut line = Gm::new(
//     Line::new(
//         &context,
//         vec2(0.0, 0.0) * scale_factor,
//         vec2(width as f32, height as f32) * scale_factor,
//         5.0 * scale_factor,
//     ),
//     ColorMaterial {
//         color: Srgba::GREEN,
//         ..Default::default()
//     },
// );

// if button == MouseButton::Left && !modifiers.ctrl {
//     rectangle.set_center(position);
// }
// if button == MouseButton::Right && !modifiers.ctrl {
//     circle.set_center(position);
// }
// if button == MouseButton::Left && modifiers.ctrl {
//     let ep = line.end_point1();
//     line.set_endpoints(position, ep);
// }
// if button == MouseButton::Right && modifiers.ctrl {
//     let ep = line.end_point0();
//     line.set_endpoints(ep, position);
// }
