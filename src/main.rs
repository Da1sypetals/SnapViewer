#![allow(warnings)]

use gl::types::*;
use nalgebra::{SVector, Vector2};
use snapviewer::render_data::{Color, RenderData, Transform};
use std::ffi::{CStr, CString};
use std::mem;
use std::ptr;
use std::time::Instant;
use thiserror::Error;

// -------------------------------------------
// 2) Error type for shader compilation/linking
// -------------------------------------------
#[derive(Debug, Error)]
enum ShaderError {
    #[error("Shader compilation failed: {0}")]
    CompileError(String),
    #[error("Shader linking failed: {0}")]
    LinkError(String),
}

// --------------------------------------------------------------
// 3) Utility: compile a single shader, or link into a program
// --------------------------------------------------------------
unsafe fn compile_shader(src: &CStr, kind: GLenum) -> Result<GLuint, ShaderError> {
    let shader = gl::CreateShader(kind);
    gl::ShaderSource(shader, 1, &src.as_ptr(), ptr::null());
    gl::CompileShader(shader);

    // Check compile status
    let mut status = GLint::from(gl::FALSE);
    gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);
    if status != GLint::from(gl::TRUE) {
        // Get the length of the info log
        let mut len = 0;
        gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
        // Grab the log
        let mut buf: Vec<u8> = Vec::with_capacity(len as usize);
        buf.extend([b' '].iter().cycle().take(len as usize));
        let error_ptr = buf.as_mut_ptr() as *mut GLchar;
        gl::GetShaderInfoLog(shader, len, ptr::null_mut(), error_ptr);
        let error = CStr::from_ptr(error_ptr).to_string_lossy().into_owned();
        gl::DeleteShader(shader);
        return Err(ShaderError::CompileError(error));
    }
    Ok(shader)
}

unsafe fn link_program(vs: GLuint, fs: GLuint) -> Result<GLuint, ShaderError> {
    let prog = gl::CreateProgram();
    gl::AttachShader(prog, vs);
    gl::AttachShader(prog, fs);
    gl::LinkProgram(prog);

    // Check link status
    let mut status = GLint::from(gl::FALSE);
    gl::GetProgramiv(prog, gl::LINK_STATUS, &mut status);
    if status != GLint::from(gl::TRUE) {
        let mut len = 0;
        gl::GetProgramiv(prog, gl::INFO_LOG_LENGTH, &mut len);
        let mut buf: Vec<u8> = Vec::with_capacity(len as usize);
        buf.extend([b' '].iter().cycle().take(len as usize));
        let error_ptr = buf.as_mut_ptr() as *mut GLchar;
        gl::GetProgramInfoLog(prog, len, ptr::null_mut(), error_ptr);
        let error = CStr::from_ptr(error_ptr).to_string_lossy().into_owned();
        gl::DeleteProgram(prog);
        return Err(ShaderError::LinkError(error));
    }
    // Once linked, shaders can be deleted
    gl::DetachShader(prog, vs);
    gl::DetachShader(prog, fs);
    gl::DeleteShader(vs);
    gl::DeleteShader(fs);

    Ok(prog)
}

// ---------------------------------------------------------------------
// 4) Our “Renderer” struct: holds shader‐program, VAO, VBO, and locations
// ---------------------------------------------------------------------
struct Renderer {
    program: GLuint,
    vao: GLuint,
    vbo: GLuint,
    vertex_count: i32, // total number of vertices
    // Uniform locations:
    loc_uScale: GLint,
    loc_uTranslate: GLint,
}

impl Renderer {
    /// Create GPU resources from a given RenderData. We upload VBO once.
    unsafe fn new(render_data: &RenderData) -> Result<Self, ShaderError> {
        // 4.1) Compile shaders (vertex uses vec2 for aPos)
        let vs_src = CString::new(
            r#"
            #version 330 core
            layout(location = 0) in vec2 aPos;
            layout(location = 1) in vec4 aColor;
            uniform vec2 uScale;
            uniform vec2 uTranslate;
            out vec4 vColor;
            void main() {
                vec2 scaled = aPos * uScale;
                vec2 translated = scaled + uTranslate;
                gl_Position = vec4(translated, 0.0, 1.0);
                vColor = aColor;
            }
        "#,
        )
        .unwrap();

        let fs_src = CString::new(
            r#"
            #version 330 core
            in vec4 vColor;
            out vec4 FragColor;
            void main() {
                FragColor = vColor;
            }
        "#,
        )
        .unwrap();

        let vs = compile_shader(&vs_src, gl::VERTEX_SHADER)?;
        let fs = compile_shader(&fs_src, gl::FRAGMENT_SHADER)?;
        let program = link_program(vs, fs)?;

        // 4.2) Prepare the interleaved vertex buffer.
        // Each triangle has 3 verts; each vert is 2 × f64 (x, y),
        // plus one Color (SVector<f32,4>) per triangle, duplicated per‐vertex.
        let num_triangles = (render_data.verts.len() / 6) as usize; // 6 f64 = 3 verts*2 coords
        let mut interleaved_data: Vec<f32> = Vec::with_capacity(num_triangles * 3 * (2 + 4));
        for tri_idx in 0..num_triangles {
            // Read triangle i's 6 f64‐coords: (x1,y1, x2,y2, x3,y3)
            let base = tri_idx * 6;
            let col = render_data.colors[tri_idx]; // [r,g,b,a] (f32)
            for v in 0..3 {
                let vx = render_data.verts[base + v * 2 + 0] as f32;
                let vy = render_data.verts[base + v * 2 + 1] as f32;
                interleaved_data.push(vx);
                interleaved_data.push(vy);
                // push the triangle‐color (same for all 3 verts)
                interleaved_data.push(col[0]);
                interleaved_data.push(col[1]);
                interleaved_data.push(col[2]);
                interleaved_data.push(col[3]);
            }
        }

        let vertex_count = (num_triangles * 3) as i32;

        // 4.3) Generate VAO + VBO and upload
        let mut vao = 0;
        let mut vbo = 0;
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);

        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (interleaved_data.len() * mem::size_of::<f32>()) as GLsizeiptr,
            interleaved_data.as_ptr() as *const _,
            gl::STATIC_DRAW,
        );

        // Layout of interleaved_data:
        // position = 2 × f32, color = 4 × f32
        let stride = (6 * mem::size_of::<f32>()) as GLsizei;
        // aPos: location = 0, 2 × float, offset = 0
        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stride, ptr::null());
        // aColor: location = 1, 4 × float, offset = 2 * sizeof(f32)
        gl::EnableVertexAttribArray(1);
        gl::VertexAttribPointer(
            1,
            4,
            gl::FLOAT,
            gl::FALSE,
            stride,
            (2 * mem::size_of::<f32>()) as *const _,
        );

        // Unbind VAO (optional)
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindVertexArray(0);

        // 4.4) Query uniform locations
        let loc_uScale = gl::GetUniformLocation(program, CString::new("uScale").unwrap().as_ptr());
        let loc_uTranslate =
            gl::GetUniformLocation(program, CString::new("uTranslate").unwrap().as_ptr());

        Ok(Renderer {
            program,
            vao,
            vbo,
            vertex_count,
            loc_uScale,
            loc_uTranslate,
        })
    }

    /// Call every frame. We bind the program, bind VAO, upload uniforms, draw.
    unsafe fn render(&self, transform: &Transform) {
        gl::UseProgram(self.program);
        gl::BindVertexArray(self.vao);

        // Upload uniforms (cast f64 → f32 here):
        gl::Uniform2f(
            self.loc_uScale,
            transform.scale.x as f32,
            transform.scale.y as f32,
        );
        gl::Uniform2f(
            self.loc_uTranslate,
            transform.translate.x as f32,
            transform.translate.y as f32,
        );

        // Draw all triangles:
        gl::DrawArrays(gl::TRIANGLES, 0, self.vertex_count);

        // Unbind (optional)
        gl::BindVertexArray(0);
        gl::UseProgram(0);
    }
}

// -------------------------------
// 5) MAIN: create window + context
// -------------------------------
fn main() {
    // 5.1) Build example RenderData with 2D verts (6 elements = 3 verts * 2 coords)
    let render_data = {
        // One triangle in 2D: (x,y) pairs
        let verts: Vec<f64> = vec![
            // T1
            -0.5, -0.5, // V0
            0.5, -0.5, // V1
            0.0, 0.0, // V2
            // T2
            -0.5, 0.5, // V0
            0.5, 0.5, // V1
            0.0, 0.0, // V2
        ];
        // One RGBA color for this triangle
        let colors = vec![
            Color::new(0.8, 0.3, 0.2, 1.0),
            Color::new(0.2, 0.8, 0.3, 1.0),
        ];

        // Initial transform = identity
        let transform = Transform::identity();

        RenderData {
            verts,
            colors,
            transform,
        }
    };

    // 5.2) Initialize a glutin window + OpenGL context
    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_title("Tomi Viewer")
        .with_inner_size(glutin::dpi::LogicalSize::new(800.0, 600.0));

    // Request an OpenGL 3.3 Core profile context
    let windowed_context = glutin::ContextBuilder::new()
        .with_vsync(true)
        .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (3, 3)))
        .with_gl_profile(glutin::GlProfile::Core)
        .build_windowed(wb, &el)
        .unwrap();

    // Make it current
    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    // Load OpenGL function pointers
    gl::load_with(|symbol| windowed_context.context().get_proc_address(symbol) as *const _);

    // Enable depth test (optional for 2D; not strictly needed here)
    unsafe {
        gl::Enable(gl::DEPTH_TEST);
        gl::DepthFunc(gl::LESS);

        // Set clear color to dark gray
        gl::ClearColor(0.2, 0.2, 0.2, 1.0);
    }

    // 5.3) Create the Renderer (upload VBO/VAO once)
    let renderer = unsafe {
        match Renderer::new(&render_data) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to initialize renderer: {}", e);
                return;
            }
        }
    };

    // 5.4) Enter the event‐loop / render loop
    let mut last_frame = Instant::now();
    let mut angle: f64 = 0.0; // animate transform

    el.run(move |event, _, control_flow| {
        // Poll continuously
        *control_flow = glutin::event_loop::ControlFlow::Poll;

        match event {
            glutin::event::Event::LoopDestroyed => return,
            glutin::event::Event::WindowEvent { event, .. } => match event {
                // Close on window close or ESC
                glutin::event::WindowEvent::CloseRequested => {
                    *control_flow = glutin::event_loop::ControlFlow::Exit
                }
                glutin::event::WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(glutin::event::VirtualKeyCode::Escape) = input.virtual_keycode {
                        if input.state == glutin::event::ElementState::Pressed {
                            *control_flow = glutin::event_loop::ControlFlow::Exit
                        }
                    }
                }
                _ => {}
            },
            glutin::event::Event::MainEventsCleared => {
                // Time‐step
                let now = Instant::now();
                let dt = now.duration_since(last_frame);
                last_frame = now;

                let mut current_transform = Transform::identity();

                // // Animate transform: e.g. scale + translate over time
                // angle += dt.as_secs_f64() * std::f64::consts::PI * 0.25; // 0.25π rad/sec
                // let scale = Vector2::new(0.5 + 0.5 * angle.cos(), 0.5 + 0.5 * angle.sin());
                // let translate = Vector2::new(0.5 * angle.cos(), 0.5 * angle.sin());
                // current_transform.scale = scale;
                // current_transform.translate = translate;

                // Clear and render
                unsafe {
                    gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
                    renderer.render(&current_transform);
                }

                windowed_context.swap_buffers().unwrap();
            }
            _ => {}
        }
    });
}
