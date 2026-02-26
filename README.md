# Snapviewer (macOS)

A PyTorch memory snapshot viewer alternative to https://docs.pytorch.org/memory_viz with rich features. Display large snapshots smoothly! 

![alt text](snapviewer.gif)

## Usage (macOS):
> Tested on macOS Sequioa 15.7.4, Python 3.13
- You need Rust toolchain and Python installed.
- First you need a virtual environment (via `venv` or `conda`). Here we use `conda`:
  - Activate your environment
    ```bash
    conda activate base
    ```
  - Install dependencies
    ```bash
    pip install -r requirements.txt
    ```
- Compile binary
  ```bash
  cargo build --release --bin snapviewer-renderer --no-default-features
  ```

- Run

  `-rr` is for `--resolution-ratio`, used to deal with the rendering pattern of Apple's retina display.

  **Option A: Pass the `.pickle` directly.** Preprocessing artifacts are cached at `~/.snapviewer_cache/` and reused on subsequent runs.
  ```bash
  python gui.py --pickle snap/large.pickle --res 1200 500 -rr 2.0
  ```

  **Option B: Pre-process manually and pass the directory.**
  ```bash
  # 1. Convert snapshot to zip
  python convert_snap.py -i snap/large.pickle -o snap/large.zip

  # 2. Decompress — you should see allocations.json and elements.db
  unzip snap/large.zip -d ./large

  # 3. Run
  python gui.py --dir ./large --res 1200 500 -rr 2.0
  ```

> Warning: This software is in pre-alpha stage. Everything including snapshot format, data storing/loading logic is under frequent change.

    

### Controls

- Pan: WASD / Left Mouse Drag
- Zoom: Mouse Wheel
- (Ctrl + Left click) on an allocation for detailed info about it


## Troubleshoot

- If you see errors with message like `cannot open input file 'sqlite3.lib'`, enable feature flag `--features bundled-sqlite`.

## Notes
- Minimal dependency is **not** a goal.
- On macos, TKinter is required to run on main thread; while on all platforms the renderer is also required to run on main thread. This means we need multiple processes if we want to do cross platform.
- todo:
  - test this zmq-based impl on windows/linux, if it works, remove the python binding impl.
  - refactor to remove pyo3-specific workarounds, and organize the project better. This may be done by LLM
  - 