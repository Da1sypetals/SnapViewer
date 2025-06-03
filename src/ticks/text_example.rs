use three_d::renderer::*;
use three_d::*;

fn main() {
    let window = Window::new(WindowSettings {
        title: "Text!".to_string(),
        max_size: Some((1280, 720)),
        ..Default::default()
    })
    .unwrap();

    let context = window.gl();
    let mut camera = Camera::new_2d(window.viewport());

    println!("init");

    let text_generator = TextGenerator::new(
        include_bytes!("../../assets/JetBrainsMono-Medium.ttf"),
        0,
        30.0,
    )
    .unwrap();

    println!("Font load ok");

    let text_mesh0 = text_generator.generate("Hello, World!", TextLayoutOptions::default());

    // Create models
    let mut text0 = Gm::new(
        Mesh::new(&context, &text_mesh0),
        ColorMaterial {
            color: Srgba::RED,
            ..Default::default()
        },
    );
    text0.set_transformation(
        // scale 10x then move to screen center
        Mat4::from_translation(vec3(0.0, 0.0, 0.0)) * Mat4::from_scale(1.0),
    );

    let mut text1 = Gm::new(
        Mesh::new(&context, &text_mesh0),
        ColorMaterial {
            color: Srgba::RED,
            ..Default::default()
        },
    );
    text1.set_transformation(
        // scale 10x then move to screen center
        Mat4::from_translation(vec3(0.0, 0.0, 0.0)) * Mat4::from_scale(2.0),
    );

    println!("Text prepare ok");

    // Render loop
    window.render_loop(move |frame_input| {
        // println!("frame");
        camera.set_viewport(frame_input.viewport);
        frame_input
            .screen()
            .clear(ClearState::color_and_depth(1.0, 1.0, 1.0, 1.0, 1.0))
            .render(&camera, [&text0, &text1], &[]);
        FrameOutput::default()
    });
}
