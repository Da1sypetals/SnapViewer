import pickle
import sys
import zipfile
import json  # Import the json module

from icecream import ic
from tqdm import tqdm

ALLOCATIONS_FILE_NAME = "allocations.json"
ELEMENTS_FILE_NAME = "elements.json"


def trace2dict(device_trace):
    alloc_data = process_alloc_data(device_trace, plot_segments=False)
    allocations_over_time = alloc_data["allocations_over_time"]
    elements = alloc_data["elements"]

    allocations = allocations_over_time[:-1]

    return allocations, elements


def process_alloc_data(device_trace, plot_segments=False):
    elements = []
    initially_allocated = []
    actions = []
    addr_to_alloc = {}

    # 第一阶段：处理事件流
    alloc_actions = (
        {"alloc", "segment_alloc"} if not plot_segments else {"segment_alloc"}
    )
    free_actions = (
        {"free", "free_completed"}
        if not plot_segments
        else {"segment_free", "segment_free"}
    )

    print("Processing events")
    for idx, event in enumerate(device_trace):
        if event["action"] in alloc_actions:
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

    # 没有预分配内存块，仅处理事件流中的初始状态

    # 第三阶段：模拟时间线
    current = []
    current_data = []
    data = []
    max_size = 0
    total_mem = 0
    total_summarized_mem = 0
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
        summarized_mem["size"].append(total_summarized_mem)
        timestep += n
        for _ in range(n):
            max_at_time.append(total_mem + total_summarized_mem)

    # 处理初始分配
    print("Processing initial allocations")
    for elem in tqdm(reversed(initially_allocated)):
        # 添加到可视分配
        element = elements[elem]
        current.append(elem)
        color = elem
        data_entry = {
            "elem": elem,
            "timesteps": [timestep],
            "offsets": [total_mem],
            "size": element["size"],
            "color": color,
        }
        current_data.append(data_entry)
        data.append(data_entry)
        total_mem += element["size"]

    # 处理动作序列
    print("Processing actions")
    for elem in tqdm(actions):
        element = elements[elem]
        size = element["size"]

        # 查找当前分配
        try:
            idx = next(i for i in reversed(range(len(current))) if current[i] == elem)
        except StopIteration:
            # 新增分配
            current.append(elem)
            color = elem
            data_entry = {
                "elem": elem,
                "timesteps": [timestep],
                "offsets": [total_mem],
                "size": size,
                "color": color,
            }
            current_data.append(data_entry)
            data.append(data_entry)
            total_mem += size
            advance(1)
        else:
            # 释放分配
            removed = current_data[idx]
            removed["timesteps"].append(timestep)
            removed["offsets"].append(removed["offsets"][-1])
            del current[idx]
            del current_data[idx]

            # 调整后续块位置
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

    # 收尾处理
    for entry in tqdm(current_data):
        entry["timesteps"].append(timestep)
        entry["offsets"].append(entry["offsets"][-1])

    data.append(summarized_mem)

    return {
        "max_size": max_size,
        "allocations_over_time": data,
        "max_at_time": max_at_time,
        "summarized_mem": summarized_mem,
        "elements": elements,
    }


def get_trace(dump: dict, device_id=0):
    trace = dump["device_traces"]
    if len(trace) <= device_id:
        # print to stderr and exit
        expected = 0 if len(trace) == 1 else f"0 ~ {len(trace) - 1}"
        print(
            f"Error: device id out of range, expected {expected}, got {device_id}",
            file=sys.stderr,
        )
        exit(1)
    return trace[device_id]


def cli():
    """
    elements: 原始的 action 对象，保存了分配地址，callstack等信息，长度为n
    allocation_over_time: 用来画图的东西。其中最后一项是summary，可以忽略，长度为n+1

    """
    import argparse

    parser = argparse.ArgumentParser()
    parser.add_argument("-i", "--input", type=str, help="path to snapshot pickle")
    parser.add_argument("-o", "--output", type=str, help="output path")
    parser.add_argument("-d", "--device", type=int, default=0, help="device id")

    args = parser.parse_args()

    with open(args.input, "rb") as f:
        dump = pickle.load(f)

    trace = get_trace(dump, args.device)
    allocations, elements = trace2dict(trace)

    print("Saving trace dictionary as zip")
    with zipfile.ZipFile(args.output, "w") as zf:
        # Compress json_trace in memory and save to zip file
        allocation_bytes = json.dumps(allocations).encode("utf-8")
        elements_bytes = json.dumps(elements).encode("utf-8")
        zf.writestr(
            ALLOCATIONS_FILE_NAME,
            allocation_bytes,
            compress_type=zipfile.ZIP_DEFLATED,
        )
        zf.writestr(
            ELEMENTS_FILE_NAME,
            elements_bytes,
            compress_type=zipfile.ZIP_DEFLATED,
        )

    ic(len(allocations))
    ic(len(elements))


if __name__ == "__main__":
    cli()
