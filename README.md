# Snapviewer

A PyTorch memory snapshot viewer alternative to https://docs.pytorch.org/memory_viz with rich features. Display large snapshots smoothly!

Tested on Windows and macOS.

![alt text](assets/snapviewer.gif)

## Usage

You need Rust toolchain installed. Python is only required if you want to use `.pickle` files directly (for preprocessing).

### Run

`-r` is for `--resolution-ratio`, used to deal with the rendering pattern of Apple's retina display. You probably need `-r 2.0` if you are using MacBook.

**Option A: Pass the `.pickle` directly.** Preprocessing artifacts are cached at `~/.snapviewer_cache/` and reused on subsequent runs. Requires Python.
```bash
cargo run -r --bin snapviewer-gui -- --pickle assets/memory.pickle --res 1200 500 -r 2
```

**Option B: Pre-process manually and pass the directory.**
```bash
# 1. Convert snapshot — outputs allocations.json and elements.db under the directory
python convert_snap.py -i assets/memory.pickle -o ./snap

# 2. Run (no Python required)
cargo run -r --bin snapviewer-gui -- --dir ./snap --res 2400 1000 -r 2.0
```

See `cargo run -r --bin snapviewer-gui -- --help` for more options.

> Warning: This software is in pre-alpha stage. Everything including snapshot format, data storing/loading logic is under frequent change.

## Controls

### Renderer Window
- Pan: WASD / Left Mouse Drag
- Zoom: Mouse Wheel
- (Ctrl + Left click) on an allocation for detailed info about it

### GUI
- Ctrl+D / Ctrl+Q: Quit application
- Arrow Up/Down: Navigate command history in REPL
- Theme picker in top bar to change color scheme
- "Hide REPL" / "Show REPL" button to toggle the REPL panel


## Troubleshoot

- If you see errors with message like `cannot open input file 'sqlite3.lib'`, enable feature flag `--features bundled-sqlite`.

## Notes
- Minimal dependency is **not** a goal.
- The application uses a multi-process architecture: the GUI runs in one process and communicates with the renderer subprocess via IPC.