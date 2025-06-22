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

## Usage

### Viewer

For argument specifications, run `cargo run --release --bin viewer -- --help`.

  * **Navigation:** Use WASD for movement and scroll the mouse wheel for zooming.
  * **Interaction:**
      * Left-click an allocation to highlight it, as well as display its details (size, call stack, etc.) in stdout.
      * Right-click anywhere to show memory information (Y-coord) and timestamp (X-coord) the cursor is currently at.
  * **Highlighting:** The viewer is a REPL (output is a bit messy though). Highlight an allocation using `--show <alloc_idx>`.

### SQLite REPL (Experimental)

For argument specifications, run `cargo run --release --bin sql-repl -- --help`. Within the SQL REPL, type `--help` for available commands.

**Troubleshooting Linker Errors:** If you encounter a linker error regarding `sqlite3` not being found, either:

- Properly install `sqlite3` on your system, or
- Utilize the `bundled-sqlite` feature flag, which compiles SQLite from source (this may take some time).

## Notes

  * Minimal dependency is **not** a goal.

## TODO

  * **Feature:** Implement call stack search functionality. Consider using `ratatui` to split the window into two panes: call stack and logs on the left, and an allocation database search (SQLite-backed) on the right.
  * **Web Integration:** Embed Snapviewer into a webpage via WebAssembly (WASM).

## Developer Notes

### Local Execution

  * **Standard:**
    ```sh
    cargo run --bin viewer -- -p snap/block8_len100.zip --res 2400 1080 --log-info
    ```
  * **Large Snapshots (Release Mode):**
    ```sh
    cargo run -r --bin viewer -- -p snap/large.zip --res 2400 1080 --log-info
    ```

### Database

  * Split window using `ratatui`.
  * Left pane: Viewer log.
  * Right pane: SQL query interface.
