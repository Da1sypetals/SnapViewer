import pickle
import sys
import zipfile
import logging
from tqdm import tqdm

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
ELEMENTS_FILE_NAME = "elements.json"


def trace_to_allocation_data(device_trace):
    """
    Convert device trace into allocation timeline and elements.

    Args:
        device_trace (list): A list of memory allocation/free events for a device.

    Returns:
        tuple: (allocations, elements)
    """
    alloc_data = process_alloc_data(device_trace, plot_segments=False)
    allocations = alloc_data["allocations_over_time"][:-1]  # Exclude summarized entry
    elements = alloc_data["elements"]
    return allocations, elements


def process_alloc_data(device_trace, plot_segments=False):
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
    alloc_actions = {"segment_alloc"} if plot_segments else {"alloc", "segment_alloc"}
    free_actions = {"segment_free"} if plot_segments else {"free", "free_completed"}

    logging.info("Processing events")
    for idx, event in enumerate(device_trace):
        if event["action"] in alloc_actions:
            # Register allocation event
            elements.append(event)
            addr_to_alloc[event["addr"]] = len(elements) - 1
            actions.append(len(elements) - 1)
        elif event["action"] in free_actions:
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
        "max_size": max_size,
        "allocations_over_time": data,
        "max_at_time": max_at_time,
        "summarized_mem": summarized_mem,
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
        print(f"Warning: requested device ({device_id}) has no trace in this snapshot.")
        print(f"         Devices with trace: {devices_with_trace}")
        sys.exit(1)

    return trace[device_id]


def cli():
    """
    Command-line interface to process a snapshot and export memory trace to a zip.
    """
    import argparse

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

    # Save trace to a zip file
    logging.info("Saving trace dictionary as zip")
    with zipfile.ZipFile(args.output, "w") as zf:
        logging.info("Dumping allocations to byte stream")
        allocation_bytes = json.dumps(allocations)

        logging.info("Dumping elements to byte stream")
        elements_bytes = json.dumps(elements)

        logging.info("Saving allocations")
        zf.writestr(ALLOCATIONS_FILE_NAME, allocation_bytes, compress_type=zipfile.ZIP_DEFLATED)

        logging.info("Saving elements")
        zf.writestr(ELEMENTS_FILE_NAME, elements_bytes, compress_type=zipfile.ZIP_DEFLATED)

    print("Trace lengths:")
    print(f"    {len(allocations) = }")
    print(f"    {len(elements) = }")


if __name__ == "__main__":
    cli()
