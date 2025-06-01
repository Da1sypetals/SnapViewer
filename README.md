# Snapviewer

Display large snapshots smoother! 

![alt text](trace.png)

## Preprocess
```sh
python parse_dump.py -p snapshots/large/transformer.pickle -o ./dumpjson -d 0 -z
```

## Use
- See `cargo run -- --help`. Please note that CLI options `-z` and `-j` conflicts.
- Navigate with WASD and mouse scroll.
- Click on the allocation, and its details (size, call stack, etc.) will show in stdout;




# TODO:
...

# Notes
Run local for me: 
```sh
cargo run -- -z snap/small.zip --res 2400 1080
```
For very large snapshots, run on release:
```sh
cargo run -r -- -z snap/transformer.zip --log-info --res 2400 1080 
```