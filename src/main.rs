use anyhow::Result as AnyhowResult;
use clap::Parser;
use log::info;
use nalgebra::Vector2;
use snapviewer::{
    database::sqlite::AllocationDatabase,
    load::read_allocations,
    render_loop::{FpsTimer, RenderLoop},
    ticks::TickGenerator,
    ui::{TranslateDir, WindowTransform},
    utils::{format_bytes_precision, get_spinner, memory_usage},
};
use std::sync::Arc;
use three_d::{
    ClearState, ColorMaterial, CpuMesh, Event, FrameOutput, Gm, Mesh, MouseButton, Srgba, Window,
    WindowSettings,
};

/// SnapViewer Renderer - Standalone OpenGL renderer with ZeroMQ IPC
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory containing allocations.json and elements.db
    #[arg(short, long)]
    dir: String,

    /// Resolution width and height
    #[arg(long, value_name = "WIDTH HEIGHT", num_args = 2, default_values_t = [2400, 1000])]
    res: Vec<u32>,

    /// ZeroMQ PUB socket port (Renderer -> UI)
    #[arg(long, default_value_t = 5555)]
    pub_port: u16,

    /// ZeroMQ REP socket port (UI -> Renderer)
    #[arg(long, default_value_t = 5556)]
    rep_port: u16,

    /// Log level
    #[arg(long, default_value_t = String::from("info"))]
    log: String,

    /// Resolution ratio for high-DPI displays (e.g., 2.0 for Retina)
    #[arg(long, default_value_t = 1.0)]
    resolution_ratio: f64,
}

struct RendererState {
    db_ptr: u64,
    resolution: (u32, u32),
    resolution_ratio: f64,
    pub_socket: zmq::Socket,
    rep_socket: zmq::Socket,
}

fn main() -> AnyhowResult<()> {
    let args = Args::parse();

    // Initialize logger
    let log_level = match args.log.as_str() {
        "trace" => log::LevelFilter::Trace,
        "info" => log::LevelFilter::Info,
        lvl => {
            anyhow::bail!("Expected `info` or `trace`, got {}", lvl);
        }
    };
    pretty_env_logger::formatted_builder()
        .filter_level(log_level)
        .init();

    // Validate resolution
    let resolution = match args.res.len() {
        2 => (args.res[0], args.res[1]),
        _ => anyhow::bail!("Resolution must have exactly 2 values (width height)"),
    };

    // Load allocations
    let allocs = read_allocations(&args.dir)?;

    // Load database
    let db = Box::leak(Box::new(AllocationDatabase::from_dir(&args.dir)?));
    let num_elems = db.row_count()?;

    // Data integrity check
    if allocs.len() != num_elems {
        anyhow::bail!(
            "# of allocation and elements mismatch: {} allocations, {} elements",
            allocs.len(),
            num_elems
        );
    }

    println!("Found {} entries", allocs.len());
    println!("Memory after init: {} MiB", memory_usage());

    // Create ZeroMQ context
    let context = zmq::Context::new();

    // Create PUB socket for sending click events to UI
    let pub_socket = context.socket(zmq::SocketType::PUB)?;
    let pub_endpoint = format!("tcp://*:{}", args.pub_port);
    pub_socket.bind(&pub_endpoint)?;
    println!("PUB socket bound to {}", pub_endpoint);

    // Create REP socket for receiving SQL commands from UI
    let rep_socket = context.socket(zmq::SocketType::REP)?;
    let rep_endpoint = format!("tcp://*:{}", args.rep_port);
    rep_socket.bind(&rep_endpoint)?;
    println!("REP socket bound to {}", rep_endpoint);

    // Initialize render loop
    println!(
        "Memory before initializing render loop: {} MiB",
        memory_usage()
    );
    let bar = get_spinner(&format!("Initializing render loop..."))?;
    let (render_loop, cpu_mesh) = RenderLoop::initialize(Arc::clone(&allocs), resolution)?;
    println!(
        "Memory after initializing render loop: {} MiB",
        memory_usage()
    );
    bar.finish();

    // Run render loop
    let state = RendererState {
        db_ptr: db as *mut AllocationDatabase as u64,
        resolution,
        resolution_ratio: args.resolution_ratio,
        pub_socket,
        rep_socket,
    };

    run_render_loop(state, render_loop, cpu_mesh)?;

    Ok(())
}

fn run_render_loop(
    state: RendererState,
    mut rl: RenderLoop,
    cpu_mesh: CpuMesh,
) -> AnyhowResult<()> {
    let bar = get_spinner(&format!("Initializing window and UI..."))?;
    println!(
        "Memory before render loop init work: {} MiB",
        memory_usage()
    );

    let window = Window::new(WindowSettings {
        title: "SnapViewer Renderer".to_string(),
        min_size: state.resolution,
        max_size: Some(state.resolution),
        ..Default::default()
    })?;
    let context = window.gl();

    info!("Moving mesh to GPU...");
    let mesh: Gm<Mesh, ColorMaterial> = Gm::new(
        Mesh::new(&context, &cpu_mesh),
        ColorMaterial {
            color: Srgba::WHITE,
            ..Default::default()
        },
    );

    drop(cpu_mesh);

    info!("Setting up window and UI...");

    // Window transformation
    let mut win_trans = WindowTransform::new(state.resolution, state.resolution_ratio);
    win_trans.set_zoom_limits(0.75, (rl.trace_geom.max_time as f32 / 100.0).max(2.0));
    let resolution_ratio = state.resolution_ratio; // Store for use in render loop

    // Ticks
    let tickgen = TickGenerator::jbmono(state.resolution, 20.0);

    // FPS timer
    let mut timer = FpsTimer::new();

    // Drag state for mouse-drag panning (start/end position approach)
    let mut dragging = false;
    let mut drag_start_mouse_pos: (f32, f32) = (0.0, 0.0);  // physical pixels
    let mut drag_start_center: Vector2<f32> = Vector2::new(0.0, 0.0);

    bar.finish();

    println!("Memory at start of render loop: {} MiB", memory_usage());

    let RendererState {
        db_ptr,
        resolution: _,
        resolution_ratio: _,
        pub_socket,
        rep_socket,
    } = state;

    window.render_loop(move |frame_input| {
        let resolution_ratio = resolution_ratio; // Force move into closure

        // Handle incoming ZeroMQ messages (non-blocking)
        if let Ok(bytes) = rep_socket.recv_bytes(zmq::DONTWAIT) {
            let command = String::from_utf8_lossy(&bytes);
            let response = match handle_sql_command(db_ptr, &command) {
                Ok(result) => result,
                Err(e) => format!("(!) SQL execution Error\n{}", e),
            };
            let _ = rep_socket.send(response.as_bytes(), 0);
        }

        // Handle events
        for event in frame_input.events.iter() {
            match *event {
                Event::MousePress {
                    button, position, modifiers, ..
                } => {
                    match button {
                        MouseButton::Left => {
                            if modifiers.ctrl {
                                // Show allocation detail
                                info!("Left click window pos: ({}, {})", position.x, position.y);
                                let cursor_world_pos = win_trans.screen2world_physical(position.into());
                                info!(
                                    "Left click world pos: ({}, {})",
                                    cursor_world_pos.x, cursor_world_pos.y
                                );

                                let alloc_idx = rl.trace_geom.find_by_pos(cursor_world_pos);
                                info!("Find by pos results: alloc id: {:?}", alloc_idx);

                                if let Some(idx) = alloc_idx {
                                    let msg = format!(
                                        "Allocation #{}\n{}",
                                        idx,
                                        rl.allocation_info(db_ptr, idx)
                                    );

                                    // Send to UI via ZeroMQ
                                    let _ = pub_socket.send(msg.as_bytes(), 0);

                                    rl.show_alloc(&context, idx);
                                }
                            } else {
                                // Start dragging - record start positions
                                dragging = true;
                                drag_start_mouse_pos = (position.x, position.y);
                                drag_start_center = win_trans.center;
                            }
                        }
                        MouseButton::Right => {
                            info!("Right click window pos: ({}, {})", position.x, position.y);
                            let cursor_world_pos = win_trans.screen2world_physical(position.into());
                            info!(
                                "Right click world pos: ({}, {})",
                                cursor_world_pos.x, cursor_world_pos.y
                            );

                            let indent = "\n    ";
                            let msg = format!(
                                "Cursor is at :{}memory: {}{}timestamp: {}",
                                indent,
                                format_bytes_precision(
                                    rl.trace_geom.yworld2memory(cursor_world_pos.y),
                                    3
                                ),
                                indent,
                                rl.trace_geom.xworld2timestamp(cursor_world_pos.x),
                            );

                            // Send to UI via ZeroMQ
                            let _ = pub_socket.send(msg.as_bytes(), 0);
                        }
                        MouseButton::Middle => {}
                    }
                }
                Event::MouseWheel {
                    delta, position, ..
                } => {
                    if delta.1 > 0.0 {
                        win_trans.zoom_in(position.into());
                    } else if delta.1 < 0.0 {
                        win_trans.zoom_out(position.into());
                    }
                }
                Event::KeyPress { kind, .. } => match kind {
                    three_d::Key::W => win_trans.translate(TranslateDir::Up),
                    three_d::Key::A => win_trans.translate(TranslateDir::Left),
                    three_d::Key::S => win_trans.translate(TranslateDir::Down),
                    three_d::Key::D => win_trans.translate(TranslateDir::Right),
                    key => {
                        info!("{:?},", key);
                    }
                },
                Event::MouseMotion { position, .. } => {
                    if dragging {
                        let ratio = resolution_ratio as f32;
                        let scale = win_trans.scale();
                        // Calculate mouse displacement in logical pixels, then scale to world coords
                        let dx = (position.x - drag_start_mouse_pos.0) / ratio * scale;
                        let dy = (position.y - drag_start_mouse_pos.1) / ratio * scale;
                        // New center = start center - displacement (dragging left moves view right)
                        win_trans.center.x = drag_start_center.x - dx;
                        win_trans.center.y = drag_start_center.y - dy;
                        win_trans.enforce_boundaries();
                    }
                }
                Event::MouseRelease { button, .. } => {
                    if button == MouseButton::Left {
                        dragging = false;
                    }
                }
                Event::MouseLeave => {
                    dragging = false;
                }
                _ => {}
            }
        }

        let cam = win_trans.camera(frame_input.viewport);

        let high_bytes = rl.trace_geom.yworld2memory(win_trans.ytop_world());
        let low_bytes = rl.trace_geom.yworld2memory(win_trans.ybot_world());
        let ticks = tickgen.generate_memory_ticks(
            low_bytes,
            high_bytes,
            win_trans.scale(),
            win_trans.center,
            &context,
        );

        let mut allocation_meshes = vec![&mesh];
        if let Some(selected_mesh) = &mut rl.selected_mesh {
            selected_mesh.material = rl.decaying_color.material();
            allocation_meshes.push(selected_mesh);
        }

        frame_input
            .screen()
            .clear(ClearState::color_and_depth(1.0, 1.0, 1.0, 1.0, 1.0))
            .render(cam, ticks.iter().chain(allocation_meshes), &[]);

        timer.tick();
        rl.decaying_color.tick(frame_input.elapsed_time / 1000.0);

        FrameOutput::default()
    });

    Ok(())
}

fn handle_sql_command(db_ptr: u64, command: &str) -> AnyhowResult<String> {
    let db = unsafe { &mut *(db_ptr as *mut AllocationDatabase) };
    let command = command.trim();

    if command.is_empty() {
        return Ok(String::new());
    }

    if command.starts_with("--") {
        return Ok(format!("Unexpected special command: {}", command));
    }

    match db.execute(command) {
        Ok(output) => Ok(format!("SQL execution OK\n{}", output)),
        Err(e) => Ok(format!("(!) SQL execution Error\n{}", e)),
    }
}
