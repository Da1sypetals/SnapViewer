use crate::{
    allocation::Allocation,
    database::sqlite::AllocationDatabase,
    load::read_allocations,
    render_loop::{FpsTimer, RenderLoop},
    ticks::TickGenerator,
    ui::{TranslateDir, WindowTransform},
    utils::{format_bytes_precision, get_spinner, memory_usage},
};
use log::info;
use pyo3::{exceptions::PyRuntimeError, prelude::*};
use std::sync::Arc;
use three_d::{
    ClearState, ColorMaterial, CpuMesh, Event, FrameOutput, Gm, Mesh, MouseButton, Srgba, Window,
    WindowSettings,
};

#[pyclass]
pub struct SnapViewer {
    pub db_ptr: u64,
    pub dir: String,
    pub allocs: Arc<[Allocation]>,
    pub log_level: log::LevelFilter,
    pub resolution: (u32, u32),
}

#[pymethods]
impl SnapViewer {
    #[new]
    pub fn new(dir: String, resolution: (u32, u32), log_level: String) -> PyResult<Self> {
        let log_level = match log_level.as_str() {
            "trace" => log::LevelFilter::Trace,
            "info" => log::LevelFilter::Info,
            lvl => {
                return Err(PyRuntimeError::new_err(format!(
                    "Expected `info` or `trace`, got {}",
                    lvl
                )));
            }
        };

        // let allocs = read_snap(&path).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        let allocs = read_allocations(&dir).map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

        // Terrible hack, but I did not find a better way.
        let db = Box::leak(Box::new(
            AllocationDatabase::from_dir(&dir)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?,
        ));

        println!("Memory after init: {} MiB", memory_usage());

        Ok(Self {
            db_ptr: db as *mut AllocationDatabase as u64,
            allocs,
            resolution,
            log_level,
            dir,
        })
    }

    pub fn execute_sql(&self, py: Python<'_>, line: String) -> PyResult<String> {
        py.allow_threads(move || {
            // Terrible hack, but I did not find a better way.
            let db = unsafe { &mut *(self.db_ptr as *mut AllocationDatabase) };

            let command = line.trim();
            if command.len() == 0 {
                return Ok("".into());
            }

            // determine: special command or SQL command
            if command.starts_with("--") {
                // is a special command
                match command {
                    _ => Ok(format!("Unexpected special command: {}", command)),
                }
            } else {
                // is a SQL command
                match (*db).execute(command) {
                    Ok(output) => {
                        // rustfmt do not collapse
                        Ok(format!("SQL execution OK\n{}", output))
                    }
                    Err(e) => Ok(format!("(!) SQL execution Error\n{}", e)),
                }
                // Ok(format!("Echo: {}", command))
            }
        })
    }

    fn viewer(&self, py: Python<'_>, callback: PyObject) -> PyResult<()> {
        println!(
            "Memory before initializing render loop: {} MiB",
            memory_usage()
        );

        let bar = get_spinner(&format!("Initializing render loop...")).unwrap();
        let (render_loop, cpu_mesh) =
            RenderLoop::initialize(Arc::clone(&self.allocs), self.resolution)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        println!(
            "Memory after initializaing render loop: {} MiB",
            memory_usage()
        );
        bar.finish();

        py.allow_threads(move || {
            self.run_render_loop_impl(render_loop, cpu_mesh, callback);
        });

        Ok(())
    }
}

impl SnapViewer {
    pub fn allocation_info(rl: &RenderLoop, db_ptr: u64, idx: usize) -> String {
        // Terrible hack, but I did not find a better way.
        let db = unsafe { &mut *(db_ptr as *mut AllocationDatabase) };
        let header = rl.trace_geom.raw_allocs[idx].to_string();

        // Everybody told me not to use interpolated string, but this is not a security sensitive app.
        let query_result = db
            .execute(&format!("SELECT callstack FROM allocs WHERE idx = {}", idx))
            .unwrap();
        let callstack = query_result.splitn(2, "callstack:").skip(1).next().unwrap();

        format!("{}|- callstack:\n{}", header, callstack)
    }

    pub fn run_render_loop_impl(&self, mut rl: RenderLoop, cpu_mesh: CpuMesh, callback: PyObject) {
        let bar = get_spinner(&format!("Initializing window and UI...")).unwrap();
        println!(
            "Memory before render loop init work: {} MiB",
            memory_usage()
        );
        let window = Window::new(WindowSettings {
            title: "SnapViewer".to_string(),
            min_size: rl.resolution,
            max_size: Some(rl.resolution),
            ..Default::default()
        })
        .unwrap();
        let context = window.gl();

        info!("Moving mesh to GPU...");
        let mesh: Gm<Mesh, ColorMaterial> = Gm::new(
            Mesh::new(&context, &cpu_mesh),
            ColorMaterial {
                color: Srgba::WHITE, // colors are mixed (component-wise multiplied)
                ..Default::default()
            },
        );

        // manually drop to free memory
        drop(cpu_mesh);

        info!("Setting up window and UI...");

        // window transformation (moving & zooming)
        let mut win_trans = WindowTransform::new(rl.resolution);
        win_trans.set_zoom_limits(0.75, (rl.trace_geom.max_time as f32 / 100.0).max(2.0));

        // ticks
        let tickgen = TickGenerator::jbmono(rl.resolution, 20.0);

        // start a timer
        let mut timer = FpsTimer::new();
        bar.finish();

        println!("Memory at start of render loop: {} MiB", memory_usage());
        let db_ptr = self.db_ptr;
        window.render_loop(move |frame_input| {
            // render loop start

            for event in frame_input.events.iter() {
                match *event {
                    Event::MousePress {
                        button, position, ..
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
                                let alloc_idx = rl.trace_geom.find_by_pos(cursor_world_pos);
                                info!("Find by pos results: alloc id: {:?}", alloc_idx);

                                // if we found an allocation at cursor position
                                if let Some(idx) = alloc_idx {
                                    // print allocation info
                                    let msg = format!(
                                        "Allocation #{}\n{}",
                                        idx,
                                        // WTF? I believe this must be a bug of rustc. This line does not work.
                                        // This is a Copy type which should not involve any lifetime stuff.
                                        // Self::allocation_info(&rl, self.db_ptr, idx)
                                        Self::allocation_info(&rl, db_ptr, idx)
                                    );

                                    Python::with_gil(|py| {
                                        if let Err(e) = callback.call1(py, (msg,)) {
                                            eprintln!("{}", e);
                                        }
                                    });

                                    rl.show_alloc(&context, idx);
                                }
                            }
                            MouseButton::Right => {
                                let cursor_world_pos = win_trans.screen2world(position.into());
                                info!(
                                    "Right click world pos: ({}, {})",
                                    cursor_world_pos.x, cursor_world_pos.y
                                );

                                // print memory position at cursor
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

                                Python::with_gil(|py| {
                                    if let Err(e) = callback.call1(py, (msg,)) {
                                        eprintln!("{}", e);
                                    }
                                });
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
                    Event::KeyPress { kind, .. } => {
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
                .render(
                    cam,
                    // line.into_iter()
                    //     .chain(&rectangle)
                    //     .chain(&circle)
                    //     .chain(&mesh),
                    ticks.iter().chain(allocation_meshes),
                    &[],
                );

            timer.tick();
            rl.decaying_color.tick(frame_input.elapsed_time / 1000.0); // this is MS

            FrameOutput::default()
        });
    }
}

/// Export module
#[pymodule]
fn snapviewer(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<SnapViewer>()?;
    Ok(())
}
