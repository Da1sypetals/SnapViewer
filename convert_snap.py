import argparse
import logging
import os
import pickle
import sys

import pyarrow as pa
import pyarrow.parquet as pq
from halo import Halo
from tqdm import tqdm
import orjson as json

# Configure logging to output to stdout with timestamps and log level
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(levelname)s - %(message)s",
    handlers=[logging.StreamHandler(sys.stdout)],
)


# Constants for output file names
ALLOCATIONS_FILE_NAME = "allocations.json"
DATABASE_FILE_NAME = "elements.parquet"


def trace_to_allocation_data(device_trace, segments, device_id):
    """
    Convert device trace into allocation timeline and elements.

    Args:
        device_trace (list): A list of memory allocation/free events for a device.
        segments (list): Snapshot segments for supplementing initially-allocated blocks.
        device_id (int): Device index, used to filter segments.

    Returns:
        tuple: (allocations, elements)
    """
    alloc_data = process_alloc_data(device_trace, segments, device_id)
    allocations = alloc_data["allocations_over_time"][:-1]  # Exclude summarized entry
    elements = alloc_data["elements"]
    return allocations, elements


def format_callstack(frames: list) -> str:
    def format_frame(iframe: tuple[int, dict]) -> str:
        index, frame = iframe
        return f"({index}) {frame['filename']}:{frame['line']}:{frame['name']}"

    return "\n".join(map(format_frame, enumerate(frames)))


def process_alloc_data(device_trace, segments, device_id):
    """
    Processes the device trace into a structured format showing allocations over time.

    Args:
        device_trace (list): List of memory events.
        segments (list): Snapshot segments for supplementing initially-allocated blocks.
        device_id (int): Device index, used to filter segments.

    Returns:
        dict: A dictionary containing memory timeline data.
    """
    elements = []
    initially_allocated = []
    actions = []
    addr_to_alloc = {}

    # Define which actions are treated as allocations/frees
    free_actions = {"free", "free_completed"}

    logging.info("Processing events")
    for event in tqdm(device_trace):
        if event["action"] == "alloc":
            elements.append(event)
            addr_to_alloc[event["addr"]] = len(elements) - 1
            actions.append(len(elements) - 1)
        elif event["action"] in free_actions:
            if event["addr"] in addr_to_alloc:
                actions.append(addr_to_alloc[event["addr"]])
                del addr_to_alloc[event["addr"]]
            else:
                elements.append(event)
                initially_allocated.append(len(elements) - 1)
                actions.append(len(elements) - 1)

    # Supplement initially_allocated with active_allocated blocks from segments
    # that had no alloc event in the trace (they were allocated before recording started).
    # Note: raw pickle blocks don't have an 'addr' field; addresses must be computed
    # by walking the segment starting from seg["address"], matching annotate_snapshot in reference.js.
    logging.info("Supplementing initially allocated blocks from segments")
    for seg in tqdm(segments):
        if seg["device"] != device_id:
            continue
        addr = seg["address"]
        for b in seg.get("blocks", []):
            b_addr = addr
            addr += b["size"]
            if b["state"] == "active_allocated" and b_addr not in addr_to_alloc:
                element = {
                    "action": "alloc",
                    "addr": b_addr,
                    "size": b.get("requested_size", b["size"]),
                    "frames": b.get("frames", []),
                    "stream": seg["stream"],
                    "version": b.get("version", 0),
                }
                elements.append(element)
                initially_allocated.append(len(elements) - 1)

    initially_allocated.reverse()

    # Edge case: no actions but there are pre-existing allocations.
    # Push one initially_allocated elem into actions so the graph isn't blank.
    if len(actions) == 0 and len(initially_allocated) > 0:
        actions.append(initially_allocated.pop())

    # Data structures for building the memory timeline
    current = []
    current_data = []
    data = []
    max_size = 0
    total_mem = 0
    timestep = 0
    max_at_time = []

    summarized_mem = {
        "elem": "summarized",
        "timesteps": [],
        "offsets": [total_mem],
        "size": [],
        "color": 0,
    }

    def advance(n):
        nonlocal timestep
        summarized_mem["timesteps"].append(timestep)
        summarized_mem["offsets"].append(total_mem)
        summarized_mem["size"].append(0)
        timestep += n
        for _ in range(n):
            max_at_time.append(total_mem)

    def add_allocation(elem):
        nonlocal total_mem
        element_obj = elements[elem]
        size = element_obj["size"]
        current.append(elem)
        e = {
            "elem": elem,
            "timesteps": [timestep],
            "offsets": [total_mem],
            "size": size,
            "color": elem,
        }
        current_data.append(e)
        data.append(e)
        total_mem += size

    logging.info("Processing initial allocations")
    for elem in tqdm(initially_allocated):
        add_allocation(elem)

    logging.info("Processing allocation/free actions")
    for elem in tqdm(actions):
        size = elements[elem]["size"]

        # Find last occurrence of elem in current (equivalent to JS findLastIndex)
        idx = -1
        for i in reversed(range(len(current))):
            if current[i] == elem:
                idx = i
                break

        if idx == -1:
            # First time seeing this elem: allocate
            add_allocation(elem)
            advance(1)
        else:
            # Second time: free
            advance(1)
            removed = current_data[idx]
            removed["timesteps"].append(timestep)
            removed["offsets"].append(removed["offsets"][-1])
            del current[idx]
            del current_data[idx]

            if idx < len(current_data):
                for entry in current_data[idx:]:
                    entry["timesteps"].append(timestep)
                    entry["offsets"].append(entry["offsets"][-1])
                    entry["timesteps"].append(timestep + 3)
                    entry["offsets"].append(entry["offsets"][-1] - size)
                advance(3)

            total_mem -= size

        max_size = max(total_mem, max_size)

    # Close the timeline for all still-allocated blocks
    for entry in tqdm(current_data):
        entry["timesteps"].append(timestep)
        entry["offsets"].append(entry["offsets"][-1])

    data.append(summarized_mem)

    return {
        "allocations_over_time": data,
        "elements": elements,
    }


def get_trace(dump: dict, device_id: int):
    """
    Retrieve the trace for a specific device from the snapshot dump.

    Args:
        dump (dict): Parsed snapshot data.
        device_id (int): Index of the device to fetch trace for.

    Returns:
        list: Trace events for the specified device.
    """
    trace = dump["device_traces"]

    # Validate device ID
    if device_id >= len(trace):
        expected = 0 if len(trace) == 1 else f"0 ~ {len(trace) - 1}"
        logging.error(f"Error: device id out of range, expected {expected}, got {device_id}")
        sys.exit(1)

    # Warn if trace is empty
    if len(trace[device_id]) == 0:
        devices_with_trace = [i for i, tr in enumerate(trace) if len(tr) > 0]
        print(
            f"Warning: requested device ({device_id}) has no trace in this snapshot.\n"
            f"         Devices with trace: {devices_with_trace}\n"
            "         Use --device <device> to specify device index."
        )
        sys.exit(1)

    return trace[device_id]


def make_parquet(allocs, elems, parquet_path):
    """
    Write allocation metadata to a Parquet file at parquet_path.

    Args:
        allocs (list): List of allocation data
        elems (list): List of element data
        parquet_path (str): Destination path for the Parquet file.
    """
    idx_col = []
    size_col = []
    start_time_col = []
    end_time_col = []
    callstack_col = []

    for i, (alloc, elem) in tqdm(enumerate(zip(allocs, elems)), total=len(allocs)):
        idx_col.append(i)
        size_col.append(alloc["size"])
        start_time_col.append(alloc["timesteps"][0])
        end_time_col.append(alloc["timesteps"][-1])
        callstack_col.append(format_callstack(elem["frames"]))

    schema = pa.schema([
        ("idx", pa.int64()),
        ("size", pa.int64()),
        ("start_time", pa.int64()),
        ("end_time", pa.int64()),
        ("callstack", pa.string()),
    ])
    table = pa.Table.from_pydict(
        {
            "idx": idx_col,
            "size": size_col,
            "start_time": start_time_col,
            "end_time": end_time_col,
            "callstack": callstack_col,
        },
        schema=schema,
    )
    pq.write_table(table, parquet_path)


def convert_pickle_to_dir(pickle_path: str, output_dir: str, device_id: int = 0):
    """
    Process a pickle file and write allocations.json + elements.parquet to output_dir.
    output_dir must already exist.
    """
    with Halo(text="Loading pickle file, this may take minutes...", spinner="dots"):
        with open(pickle_path, "rb") as f:
            dump = pickle.load(f)
        trace = get_trace(dump, device_id)

    with Halo(text="Processing trace data, this may take minutes...", spinner="dots"):
        allocations, elements = trace_to_allocation_data(trace, dump["segments"], device_id)

    make_parquet(allocations, elements, os.path.join(output_dir, DATABASE_FILE_NAME))

    with Halo(text="Serializing allocations to JSON, this may take minutes...", spinner="dots"):
        alloc_bytes = json.dumps(allocations)

        with open(os.path.join(output_dir, ALLOCATIONS_FILE_NAME), "wb") as f:
            f.write(alloc_bytes)


def cli():
    """
    Command-line interface to process a snapshot and write allocations.json + elements.db to a directory.
    """
    parser = argparse.ArgumentParser()
    parser.add_argument("-i", "--input", required=True, type=str, help="Path to snapshot pickle")
    parser.add_argument("-o", "--output", required=True, type=str, help="Output directory path")
    parser.add_argument("-d", "--device", type=int, default=0, help="Device ID (default=0)")
    args = parser.parse_args()

    os.makedirs(args.output, exist_ok=True)
    convert_pickle_to_dir(args.input, args.output, args.device)

    print("Done.")
    print(f"Output written to: {args.output}")
    print(f"  {os.path.join(args.output, ALLOCATIONS_FILE_NAME)}")
    print(f"  {os.path.join(args.output, DATABASE_FILE_NAME)}")


if __name__ == "__main__":
    cli()
