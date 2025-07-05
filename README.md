# Snapviewer

A PyTorch memory snapshot viewer alternative to https://docs.pytorch.org/memory_viz. Display large snapshots smoothly! 

![alt text](snapviewer.gif)

## Preprocessing

1.  **Record Snapshot:** Generate a memory snapshot of your PyTorch model. Refer to the [official PyTorch documentation](https://www.google.com/search?q=https://pytorch.org/docs/stable/cuda.html%23memory-management) for detailed instructions.

2.  **Convert to ZIP:** Convert the `.pickle` snapshot to a `.zip` format (compressed json) using `convert_snap.py`.

    ```sh
    # Install dependencies
    pip install -r requirements.txt

    # Convert snapshot
    python convert_snap.py -i snap/large.pickle -o snap/large.zip
    ```

## Usage:
- You need Rust toolchain and Python installed.
- First you need a virtual environment (via `venv` or `conda`). Here we use venv on windows:
  - Activate `venv`
    ```powershell
    .\.venv\Scripts\activate
    ```
- If you don't have `maturin` on your environment, run
  ```sh
  pip install maturin
  ```
- Build the extension with
  ```sh
  maturin dev -r
  ```
- Specify resolution, log level and path to your snapshot, and run the application.
  - QT application: (only tested on linux)
    ```sh
    pip install textual==3.5.0
    python qt.py --log info --res 2400 1000 -p <path_to_your_snapshot>
    ```
  - Textual TUI: (works on windows, has compatibility issues on linux)
    ```sh
    pip install pyqt6
    python tui.py --log info --res 2400 1000 -p <path_to_your_snapshot>
    ```

> Tested on Windows and Linux.


## Notes
- Minimal dependency is **not** a goal.
