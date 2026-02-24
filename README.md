# Snapviewer

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

## Usage:
- You need Rust toolchain and Python installed.
- First you need a virtual environment (via `venv` or `conda`). Here we use venv on windows:
  - Activate `venv`
    ```powershell
    .\.venv\Scripts\activate
    ```
  - Install dependencies
    ```powershell
    pip install -r requirements.txt
    ```
- Build the extension under release mode
  ```sh
  maturin dev -r
  ```
- Specify resolution, log level and directory to your snapshot (which should have `allocations.json` and `elements.db`), and run the application.
  - Tkinter application (tested on windows and linux)
    ```sh
    python gui.py --log info --res 2400 1000 -d <dir_to_your_snapshot>
    ```

### Controls

- Pan: WASD / Arrow Keys
- Zoom: Mouse Wheel
- Left click on an allocation for detailed info about it

## Notes
- Minimal dependency is **not** a goal.
- todo:
  - reorganize where each function and struct should be placed
  - clean code and rename functions