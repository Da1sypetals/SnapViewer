use crate::{
    allocation::Allocation, database::sqlite::AllocationDatabase, geometry::TraceGeometry,
    render_data, utils::memory_usage,
};
use std::sync::Arc;
use three_d::{ColorMaterial, Context, CpuMesh, Gm, Mesh, Srgba};

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
            log::trace!("FPS: {:.2}", self.frame as f64 / elapsed as f64);
            self.timer = std::time::Instant::now();
            self.frame = 0;
        }
    }
}

pub struct DecayingColor {
    pub fade_time: f64,
    pub time: f64,
    pub material: ColorMaterial,
    pub target_color: Srgba,
}

impl DecayingColor {
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
    pub decaying_color: DecayingColor,
    pub alloc_colors: Vec<Srgba>,
}

impl RenderLoop {
    /// Executed at start
    pub fn initialize(
        allocations: Arc<[Allocation]>,
        resolution: (u32, u32),
    ) -> anyhow::Result<(Self, CpuMesh)> {
        println!("Memory before building geometry: {} MiB", memory_usage());
        let trace_geom = TraceGeometry::from_allocations(Arc::clone(&allocations), resolution);
        println!("Memory after building geometry: {} MiB", memory_usage());
        let (cpumesh, alloc_colors) = render_data::from_allocations(trace_geom.allocations.iter());
        println!("Memory after building render data: {} MiB", memory_usage());

        Ok((
            Self {
                trace_geom,
                resolution,
                selected_mesh: None,
                decaying_color: DecayingColor::new(0.8, Srgba::WHITE),
                alloc_colors,
            },
            cpumesh,
        ))
    }

    pub fn show_alloc(&mut self, context: &Context, idx: usize) {
        // animate allocated mesh
        let (cpu_mesh, _) = render_data::from_allocations_with_z(
            std::iter::once((&self.trace_geom.allocations[idx], Srgba::WHITE)),
            0.005,
        );
        let alloc_mesh = Gm::new(
            Mesh::new(&context, &cpu_mesh),
            self.decaying_color.material(),
        );
        self.selected_mesh = Some(alloc_mesh);

        // The original color of the allocation
        let original_color = self.alloc_colors[idx];
        self.decaying_color.reset(original_color);
    }

    pub fn allocation_info(&self, db_ptr: u64, idx: usize) -> String {
        // Terrible hack, but I did not find a better way.
        let db = unsafe { &mut *(db_ptr as *mut AllocationDatabase) };
        let header = self.trace_geom.raw_allocs[idx].to_string();

        // Everybody told me not to use interpolated string, but this is not a security sensitive app.
        let query_result = db
            .execute(&format!("SELECT callstack FROM allocs WHERE idx = {}", idx))
            .unwrap();
        let callstack = query_result.splitn(2, "callstack:").skip(1).next().unwrap();

        format!("{}|- callstack:\n{}", header, callstack)
    }
}
