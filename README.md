# Snapviewer (macOS)

A PyTorch memory snapshot viewer alternative to https://docs.pytorch.org/memory_viz with rich features. Display large snapshots smoothly! 

![alt text](snapviewer.gif)

## Preprocessing

1.  **Record Snapshot:** Generate a memory snapshot of your PyTorch model. Refer to the [official PyTorch documentation](https://docs.pytorch.org/docs/stable/torch_cuda_memory.html) for detailed instructions.

2.  **Convert to ZIP:** Convert the `.pickle` snapshot to a `.zip` format (compressed json) using `convert_snap.py`.

    ```sh
    # Install dependencies
    pip install -r requirements.txt

    # Convert snapshot
    python convert_snap.py -i snap/large.pickle -o snap/large.zip
    ```
3.  **Decompress converted snapshot**: Unzip the snapshot to certain folder, say, `./large`. You should see two files, namely `allocations.json` and `elements.db`.
    ```sh
    unzip snap/large.zip -d ./large
    ```
> Warning: This software is in pre-alpha stage. Everything including snapshot format, data storing/loading logic is under frequent change.

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
  ```bash
  python gui.py --dir snap/ --res 1200 500 -rr 2.0
  ```

    

### Controls

- Pan: WASD / Arrow Keys
- Zoom: Mouse Wheel
- Left click on an allocation for detailed info about it

## Notes
- Minimal dependency is **not** a goal.
- On macos, TKinter is required to run on main thread; while on all platforms the renderer is also required to run on main thread. This means we need multiple processes if we want to do cross platform.
- todo:
  - test this zmq-based impl on windows/linux, if it works, remove the python binding impl.