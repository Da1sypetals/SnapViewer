def format_size(num):
    units = ["", "Ki", "Mi", "Gi", "Ti", "Pi", "Ei", "Zi"]
    for unit in units:
        if abs(num) < 1024.0:
            return f"{num:.3f}{unit}B"
        num /= 1024.0
    return f"{num:.1f}YiB"


def get_intervals():
    return list(map(lambda x: 4**x, range(16)))


def choose_interval(a, b, min_ticks):
    span = abs(b - a)
    if span == 0:
        return 1
    intervals = get_intervals()
    valid_intervals = [i for i in intervals if (span / i) > min_ticks]
    if not valid_intervals:
        return min(intervals)
    return max(valid_intervals)


def generate_ticks(a, b, interval):
    min_val, max_val = min(a, b), max(a, b)
    ticks = []
    i = 0
    while True:
        tick = i * interval
        if tick > max_val:
            break
        if tick >= min_val:
            ticks.append(tick)
        i += 1
    return ticks


def memory_ticks(a, b, min_ticks=8):
    interval = choose_interval(a, b, min_ticks)
    ticks = generate_ticks(a, b, interval)
    return ticks


if __name__ == "__main__":
    ticks = memory_ticks(1244, 23509823)
    print(ticks)

    ticks = memory_ticks(121244, 239823)
    print(ticks)
