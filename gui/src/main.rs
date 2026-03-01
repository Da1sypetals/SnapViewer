mod app;
mod font;
mod ipc_worker;
mod palette;

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, bail};
use clap::{ArgGroup, Parser};
use ipc_channel::ipc::IpcOneShotServer;

use app::SnapViewerApp;
use palette::PaletteName;

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug, Clone)]
#[command(
    author,
    version,
    about = "SnapViewer - Memory Allocation Viewer & SQLite REPL"
)]
#[command(group(ArgGroup::new("source").required(true).args(["dir", "pickle"])))]
pub struct Args {
    /// Path to the renderer binary. Skips auto-detection and cargo build fallback.
    #[arg(long)]
    bin: Option<PathBuf>,

    /// Logging level for the renderer.
    #[arg(long, default_value = "info", value_parser = ["info", "trace"])]
    log: String,

    /// Resolution as WIDTH HEIGHT (passed to the renderer).
    #[arg(long = "res", num_args = 2, default_values = ["2400", "1000"], value_names = ["WIDTH", "HEIGHT"])]
    resolution: Vec<u32>,

    /// Resolution ratio for high-DPI displays (e.g. 2.0 for Retina).
    #[arg(short = 'r', long = "resolution-ratio", default_value_t = 1.0)]
    resolution_ratio: f32,

    /// Color theme.
    #[arg(long, default_value = "default")]
    theme: PaletteName,

    /// Directory containing allocations.json and elements.db.
    #[arg(short = 'd', long)]
    dir: Option<PathBuf>,

    /// Path to a .pickle snapshot. Preprocessing result is cached under ~/.snapviewer_cache/.
    #[arg(long)]
    pickle: Option<PathBuf>,

    /// Device ID to use when --pickle is provided.
    #[arg(long, default_value_t = 0)]
    device: u32,
}

// ── cache / pickle helpers ────────────────────────────────────────────────────

fn compute_file_hash(path: &Path) -> Result<String> {
    const HASH_CAP: usize = 128 * 1024 * 1024;
    let data = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let cap = data.len().min(HASH_CAP);
    let hash = blake3::hash(&data[..cap]);
    Ok(hash.to_hex().to_string())
}

const VERSION: &str = "0";

fn get_or_create_cache(pickle_path: &Path, device_id: u32) -> Result<PathBuf> {
    let cache_root = home_dir().join(".snapviewer_cache");

    let file_hash = compute_file_hash(pickle_path)?;
    let cache_key = format!("{file_hash}_dev{device_id}_v{VERSION}");
    let cache_dir = cache_root.join(&cache_key);
    let alloc_file = cache_dir.join("allocations.json");
    let db_file = cache_dir.join("elements.db");

    if alloc_file.exists() && db_file.exists() {
        println!("Cache hit:");
        println!("- version: {VERSION}");
        println!("- path:    {}", cache_dir.display());
        return Ok(cache_dir);
    }

    println!("Cache miss, converting pickle: {}", pickle_path.display());
    std::fs::create_dir_all(&cache_dir)?;

    // Delegate to the Python convert_snap.py script that lives in the
    // sibling SnapViewer/ directory relative to this project's manifest.
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or(Path::new("."))
        .join("convert_snap.py");

    let status = Command::new("python")
        .args([
            script.to_str().unwrap_or("convert_snap.py"),
            "-i",
            pickle_path.to_str().unwrap(),
            "-o",
            cache_dir.to_str().unwrap(),
            "-d",
            &device_id.to_string(),
        ])
        .status()
        .context("running convert_snap.py")?;

    if !status.success() {
        bail!("convert_snap.py failed with status {status}");
    }

    Ok(cache_dir)
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

// ── port check ────────────────────────────────────────────────────────────────

// ── renderer spawn ────────────────────────────────────────────────────────────

fn spawn_renderer(args: &Args, data_dir: &Path, bootstrap_name: String) -> Result<Child> {
    let renderer_binary: PathBuf = if let Some(bin) = &args.bin {
        if !bin.exists() {
            bail!("Renderer binary not found at {}", bin.display());
        }
        bin.clone()
    } else {
        let exe_suffix = if cfg!(windows) { ".exe" } else { "" };
        // First look in the workspace target (unified build), then in SnapViewer's own target
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap_or(Path::new("."));
        let candidates = [
            workspace_root.join(format!("target/release/snapviewer-renderer{exe_suffix}")),
            workspace_root.join(format!("target/debug/snapviewer-renderer{exe_suffix}")),
        ];

        let mut found = None;
        for path in &candidates {
            if path.exists() {
                found = Some(path.clone());
                break;
            }
        }

        if let Some(p) = found {
            p
        } else {
            println!("Renderer binary not found in expected locations, building...");
            let status = Command::new("cargo")
                .args(["build", "--release", "--bin", "snapviewer-renderer"])
                .current_dir(workspace_root)
                .status()
                .context("cargo build")?;
            if !status.success() {
                bail!("cargo build failed");
            }
            workspace_root.join(format!("target/release/snapviewer-renderer{exe_suffix}"))
        }
    };

    let cmd_args: Vec<String> = vec![
        "--dir".into(),
        data_dir.to_str().unwrap().into(),
        "--res".into(),
        args.resolution[0].to_string(),
        args.resolution[1].to_string(),
        "--resolution-ratio".into(),
        args.resolution_ratio.to_string(),
        "--ipc-bootstrap".into(),
        bootstrap_name,
        "--log".into(),
        args.log.clone(),
    ];

    println!(
        "Starting renderer process: {} {}",
        renderer_binary.display(),
        cmd_args.join(" ")
    );

    let child = Command::new(&renderer_binary)
        .args(&cmd_args)
        .spawn()
        .with_context(|| format!("spawning {}", renderer_binary.display()))?;

    Ok(child)
}

// ── entry point ───────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let args = Args::parse();

    // Resolve the data directory (from --dir or --pickle cache)
    let data_dir: PathBuf = if let Some(pickle) = &args.pickle {
        if !pickle.exists() {
            bail!("pickle file '{}' does not exist", pickle.display());
        }
        get_or_create_cache(pickle, args.device)?
    } else {
        args.dir.clone().unwrap()
    };

    if !data_dir.exists() {
        bail!("data directory '{}' does not exist", data_dir.display());
    }

    // Create IPC bootstrap server and spawn renderer
    type UiChannels = (
        ipc_channel::ipc::IpcReceiver<String>,
        ipc_channel::ipc::IpcSender<String>,
        ipc_channel::ipc::IpcReceiver<String>,
    );
    let (bootstrap_server, bootstrap_name) = IpcOneShotServer::<UiChannels>::new()?;
    let mut renderer = spawn_renderer(&args, &data_dir, bootstrap_name)?;

    // Block until renderer connects and sends channel endpoints
    let (_, (event_rx, sql_tx, reply_rx)) = bootstrap_server.accept()?;
    let event_rx = Arc::new(Mutex::new(event_rx));
    let reply_rx = Arc::new(Mutex::new(reply_rx));

    let palette = args.theme;
    let title_str = format!(
        "SnapViewer - Memory Allocation Viewer & SQLite REPL  ({})",
        data_dir.display()
    );

    // Run the iced GUI (blocks until the window is closed)
    let result = iced::application(
        move || {
            SnapViewerApp::new(
                sql_tx.clone(),
                Arc::clone(&reply_rx),
                Arc::clone(&event_rx),
                palette,
            )
        },
        SnapViewerApp::update,
        SnapViewerApp::view,
    )
    .subscription(SnapViewerApp::subscription)
    .theme(SnapViewerApp::theme)
    .title(move |_state: &SnapViewerApp| title_str.clone())
    .window_size((1600.0, 1000.0))
    .font(font::FONT_BYTES)
    .run();

    // Clean up renderer on exit
    let _ = renderer.kill();
    let _ = renderer.wait();

    result.map_err(|e| anyhow::anyhow!("{e}"))
}
