pub static UNITS: [&str; 8] = ["", "Ki", "Mi", "Gi", "Ti", "Pi", "Ei", "Zi"];

pub fn format_bytes(bytes: u64) -> String {
    let mut num = bytes as f64;

    for unit in UNITS {
        if num.abs() < 1024.0 {
            return format!("{:.1} {}B", num, unit);
        }
        num /= 1024.0;
    }

    format!("{:.1}YiB", num) // Should be unreachable for typical u64 values
}
