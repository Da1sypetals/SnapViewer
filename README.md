# Snapviewer

A PyTorch memory snapshot viewer alternative to https://docs.pytorch.org/memory_viz. Display large snapshots smoothly! 

![alt text](snapviewer.gif)

## Preprocess
- Record memory snapsnot for your model. You may refer to [documentation](https://docs.pytorch.org/docs/stable/torch_cuda_memory.html);
- Convert the snapshot to zip format with `convert_snap.py`.
```sh
# first install dependencies
pip install -r requirements.txt

# then convert snapshot format
python convert_snap.py -i snap/large.pickle -o snap/large.zip
```

## Use
- See `cargo run --bin viewer -- --help`.
- Navigate with WASD and mouse scroll.
- Left click on the allocation, and its details (size, call stack, etc.) will show in stdout;
- Right click anywhere and (memory the cursor's $y$ coords is at) will show in stdout.

### Use: SqLite repl (experimental)
- See `cargo run --bin sql-repl -- --help`.


## Notes
Minimal dependency is **not** a goal.

# TODO:
- Feature: search in the call stack.
    - Use ratatui to split window? left: call stack and logs, right: search the allocation database (sqlite?)
- Embed this in a web page via WASM.

# Notes for myself

## Run
Run local for me: 
```sh
cargo run --bin viewer -- -p snap/block8_len100.zip --res 2400 1080 --log-info
```
For very large snapshots, run on release:
```sh
cargo run -r --bin viewer -- -p snap/large.zip --res 2400 1080 --log-info
```

## Database
- split window via ratatui
- left: viewer log
- right: sql query

## Features
Optionally use bundled sqlite via a `bundled-sqlite` feature (maybe control by a cli arg?)