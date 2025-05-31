# Snapviewer

Display large snapshots smoother! Navigate with WASD and mouse scroll.

![alt text](trace.png)

## Preprocess
```sh
python parse_dump.py -p snapshots/large/transformer.pickle -o ./dumpjson -d 0 -z
```

TODO: resolve click
- given a click on the screen: 
  1. compute where the click in world coords
  2. compute which allocation the click is in


TODO: show call stack