import logging
import pickle
import sqlite3
import sys
import zipfile
import tempfile  # Import tempfile

from tqdm import tqdm, trange

# Configure logging to output to stdout with timestamps and log level
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(levelname)s - %(message)s",
    handlers=[logging.StreamHandler(sys.stdout)],
)

# try to import orjson for faster json serialization
try:
    import orjson as json

    logging.info("Using orjson")
except ImportError:
    import json

    logging.warning("Falling back to stdlib json")


# Constants for file names used in the zip output
ALLOCATIONS_FILE_NAME = "allocations.json"
DATABASE_FILE_NAME = "elements.db"
DATABASE_SCHEMA = """CREATE TABLE allocs (
    idx INTEGER PRIMARY KEY,
    size INTEGER,
    start_time INTEGER,
    end_time INTEGER,
    callstack TEXT
);"""


def trace_to_allocation_data(device_trace):
    """
    Convert device trace into allocation timeline and elements.

    Args:
        device_trace (list): A list of memory allocation/free events for a device.

    Returns:
        tuple: (allocations, elements)
    """
    alloc_data = process_alloc_data(device_trace)
    allocations = alloc_data["allocations_over_time"][:-1]  # Exclude summarized entry
    elements = alloc_data["elements"]
    return allocations, elements


def format_callstack(frames: list) -> str:
    def format_frame(iframe: tuple[int, dict]) -> str:
        index, frame = iframe
        return f"({index}) {frame['filename']}:{frame['line']}:{frame['name']}"

    return "\n".join(map(format_frame, enumerate(frames)))


def process_alloc_data(device_trace):
    """
    Processes the device trace into a structured format showing allocations over time.

    Args:
        device_trace (list): List of memory events.
        plot_segments (bool): Whether to consider segment-based allocations.

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
    for idx, event in tqdm(enumerate(device_trace)):
        if event["action"] == "alloc":
            # If current action is allocation, Register allocation event
            elements.append(event)
            addr_to_alloc[event["addr"]] = len(elements) - 1
            actions.append(len(elements) - 1)
        elif event["action"] in free_actions:
            # If current action is free
            # Handle free events, potentially unmatched ones
            if event["addr"] in addr_to_alloc:
                actions.append(addr_to_alloc[event["addr"]])
                del addr_to_alloc[event["addr"]]
            else:
                elements.append(event)
                initially_allocated.append(len(elements) - 1)
                actions.append(len(elements) - 1)

    # Data structures for building the memory timeline
    current = []
    current_data = []
    data = []
    max_size = 0
    total_mem = 0
    total_summarized_mem = 0
    timestep = 0
    max_at_time = []

    # Special summarized memory track
    summarized_mem = {
        "elem": "summarized",
        "timesteps": [],
        "offsets": [total_mem],
        "size": [],
        "color": 0,
    }

    def advance(n):
        """Advance the timeline by `n` steps, tracking summary usage."""
        nonlocal timestep
        summarized_mem["timesteps"].append(timestep)
        summarized_mem["offsets"].append(total_mem)
        summarized_mem["size"].append(total_summarized_mem)
        timestep += n
        for _ in range(n):
            max_at_time.append(total_mem + total_summarized_mem)

    logging.info("Processing initial allocations")
    for elem in tqdm(reversed(initially_allocated)):
        element = elements[elem]
        current.append(elem)
        data_entry = {
            "elem": elem,
            "timesteps": [timestep],
            "offsets": [total_mem],
            "size": element["size"],
            "color": elem,
        }
        current_data.append(data_entry)
        data.append(data_entry)
        total_mem += element["size"]

    logging.info("Processing allocation/free actions")
    for elem in tqdm(actions):
        element = elements[elem]
        size = element["size"]

        # Attempt to match element in current allocations
        try:
            idx = next(i for i in reversed(range(len(current))) if current[i] == elem)
        except StopIteration:
            # New allocation
            current.append(elem)
            data_entry = {
                "elem": elem,
                "timesteps": [timestep],
                "offsets": [total_mem],
                "size": size,
                "color": elem,
            }
            current_data.append(data_entry)
            data.append(data_entry)
            total_mem += size
            advance(1)
        else:
            # Freeing memory
            removed = current_data[idx]
            removed["timesteps"].append(timestep)
            removed["offsets"].append(removed["offsets"][-1])
            del current[idx]
            del current_data[idx]

            # Adjust offsets for elements after the removed one
            if idx < len(current_data):
                for entry in current_data[idx:]:
                    entry["timesteps"].append(timestep)
                    entry["offsets"].append(entry["offsets"][-1])
                    entry["timesteps"].append(timestep + 3)
                    entry["offsets"].append(entry["offsets"][-1] - size)
                advance(3)

            total_mem -= size
            advance(1)

        max_size = max(max_size, total_mem + total_summarized_mem)

    # Close the timeline for all still-allocated blocks
    for entry in tqdm(current_data):
        entry["timesteps"].append(timestep)
        entry["offsets"].append(entry["offsets"][-1])

    # Append summary entry to timeline
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


def make_db(allocs, elems):
    """
    Create an SQLite database in a temporary file and return the path to it.

    Args:
        allocs (list): List of allocation data
        elems (list): List of element data

    Returns:
        str: Path to the temporary SQLite database file.
    """
    logging.info("Creating temporary SQLite database file")

    # Create a temporary file
    temp_db_file = tempfile.NamedTemporaryFile(delete=False, suffix=".db")
    temp_db_path = temp_db_file.name
    temp_db_file.close()  # Close the file handle as sqlite3.connect will open it

    conn = sqlite3.connect(temp_db_path)
    cursor = conn.cursor()

    # Create table schema
    cursor.execute(DATABASE_SCHEMA)

    INSERT_BATCH_SIZE = 10000
    for i in trange(0, len(allocs), INSERT_BATCH_SIZE):
        start_idx = i
        end_idx = min(i + INSERT_BATCH_SIZE, len(allocs))

        def insert_data(idx, alloc, elem):
            return (
                idx,
                alloc["size"],
                alloc["timesteps"][0],
                alloc["timesteps"][-1],
                format_callstack(elem["frames"]),
            )

        cursor.executemany(
            "INSERT INTO allocs VALUES (?, ?, ?, ?, ?)",
            map(
                lambda x: insert_data(*x),
                zip(
                    range(start_idx, end_idx),
                    allocs[start_idx:end_idx],
                    elems[start_idx:end_idx],
                ),
            ),
        )
        conn.commit()

    conn.close()
    logging.info(f"Database written to temporary file: {temp_db_path}")
    return temp_db_path


def cli():
    """
    Command-line interface to process a snapshot and export memory trace to a zip.
    """
    import argparse
    import os  # Import os for deleting the temporary file

    parser = argparse.ArgumentParser()
    parser.add_argument("-i", "--input", required=True, type=str, help="Path to snapshot pickle")
    parser.add_argument("-o", "--output", required=True, type=str, help="Output zip file path")
    parser.add_argument("-d", "--device", type=int, default=0, help="Device ID (default=0)")
    args = parser.parse_args()

    # Load the snapshot
    with open(args.input, "rb") as f:
        dump = pickle.load(f)

    # Extract and process trace
    trace = get_trace(dump, args.device)
    allocations, elements = trace_to_allocation_data(trace)

    # Create temporary database file
    db_file_path = None
    try:
        db_file_path = make_db(allocations, elements)

        # Save trace to a zip file
        logging.info("Saving trace dictionary as zip")
        with zipfile.ZipFile(args.output, "w") as zf:
            logging.info("Dumping allocations to byte stream")
            allocation_bytes = json.dumps(allocations)

            logging.info("Saving allocations")
            zf.writestr(ALLOCATIONS_FILE_NAME, allocation_bytes, compress_type=zipfile.ZIP_DEFLATED)

            logging.info(f"Saving database from temporary file: {db_file_path}")
            zf.write(db_file_path, arcname=DATABASE_FILE_NAME, compress_type=zipfile.ZIP_DEFLATED)

    finally:
        # Clean up the temporary database file
        if db_file_path and os.path.exists(db_file_path):
            logging.info(f"Deleting temporary database file: {db_file_path}")
            os.remove(db_file_path)

    print("Trace lengths:")
    print(f"    {len(allocations) = }")
    print(f"    {len(elements) = }")


if __name__ == "__main__":
    cli()
