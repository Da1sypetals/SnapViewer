use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

pub const ALLOCATIONS_FILE_NAME: &str = "allocations.json";
pub const ELEMENTS_FILE_NAME: &str = "elements.json";
pub const UNITS: [&str; 8] = ["", "Ki", "Mi", "Gi", "Ti", "Pi", "Ei", "Zi"];
pub const INTERVALS: [f64; 16] = [
    1.0_f64,
    4.0_f64,
    16.0_f64,
    64.0_f64,
    256.0_f64,
    1024.0_f64,
    4096.0_f64,
    16384.0_f64,
    65536.0_f64,
    262144.0_f64,
    1048576.0_f64,
    4194304.0_f64,
    16777216.0_f64,
    67108864.0_f64,
    268435456.0_f64,
    1073741824.0_f64,
];

pub fn format_bytes(bytes: i64) -> String {
    let mut num = bytes as f64;
    let sign = if num < 0.0 { "-" } else { "" };

    for unit in UNITS {
        if num.abs() < 1024.0 {
            return format!("{}{:.2} {}B", sign, num, unit);
        }
        num /= 1024.0;
    }

    format!("{}{:.1}YiB", sign, num) // Should be unreachable for typical u64 values
}

/// Rust has no default parameter value ...
pub fn format_bytes_precision(bytes: i64, precision: usize) -> String {
    let mut num = bytes as f64;
    let sign = if num < 0.0 { "-" } else { "" };

    for unit in UNITS {
        if num.abs() < 1024.0 {
            return format!("{}{:.3$} {}B", sign, num, unit, precision);
        }
        num /= 1024.0;
    }

    format!("{}{:.2$} YiB", sign, num, precision) // Should be unreachable for typical u64 values
}

fn choose_interval(a: f64, b: f64, min_ticks: usize) -> f64 {
    let span = (b - a).abs();
    if span == 0.0 {
        return 1.0;
    }
    let valid_intervals: Vec<f64> = INTERVALS
        .iter()
        .filter_map(|&i| {
            if (span / i) > min_ticks as f64 {
                Some(i)
            } else {
                None
            }
        })
        .collect();

    if valid_intervals.is_empty() {
        // In Python, min(intervals) would be 4^0 = 1.0
        return INTERVALS.into_iter().next().unwrap_or(1.0);
    }
    *valid_intervals
        .iter()
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(&1.0)
}

fn generate_ticks_f64(a: f64, b: f64, interval: f64) -> Vec<f64> {
    let min_val = a.min(b);
    let max_val = a.max(b);
    let mut ticks = Vec::new();
    let mut i = 0;
    loop {
        let tick = i as f64 * interval;
        // Adding a small epsilon for floating point comparison robustness
        if tick > max_val + f64::EPSILON {
            break;
        }
        if tick >= min_val - f64::EPSILON {
            ticks.push(tick);
        }
        i += 1;
    }
    ticks
}

pub fn generate_ticks(low_bytes: i64, high_bytes: i64) -> Vec<i64> {
    let a = low_bytes as f64;
    let b = high_bytes as f64;
    let min_ticks = 8; // Default value from the Python function

    let interval = choose_interval(a, b, min_ticks);
    let ticks_f64 = generate_ticks_f64(a, b, interval);

    ticks_f64.into_iter().map(|t| t as i64).collect()
}

#[cfg(test)]
mod tests {
    use crate::utils::generate_ticks;

    #[test]
    fn test_ticks() {
        let ticks = generate_ticks(1244, 23509823);
        assert_eq!(
            ticks,
            vec![
                1048576, 2097152, 3145728, 4194304, 5242880, 6291456, 7340032, 8388608, 9437184,
                10485760, 11534336, 12582912, 13631488, 14680064, 15728640, 16777216, 17825792,
                18874368, 19922944, 20971520, 22020096, 23068672
            ]
        );

        let ticks = generate_ticks(121244, 239823);
        assert_eq!(
            ticks,
            vec![
                122880, 126976, 131072, 135168, 139264, 143360, 147456, 151552, 155648, 159744,
                163840, 167936, 172032, 176128, 180224, 184320, 188416, 192512, 196608, 200704,
                204800, 208896, 212992, 217088, 221184, 225280, 229376, 233472, 237568
            ]
        );
    }
}

pub fn get_spinner(message: &str) -> anyhow::Result<ProgressBar> {
    let bar = ProgressBar::new_spinner();

    // bar.set_style(
    //     ProgressStyle::with_template("{spinner:.white} {msg}")?.tick_strings(&[
    //         ">=======", "=>======", "==>=====", "===>====", "====>===", "=====>==", "======>=",
    //         "=======>", "=======<", "======<=", "=====<==", "====<===", "===<====", "==<=====",
    //         "=<======", "<=======", "== OK ==",
    //     ]),
    // );

    // bar.set_style(
    //     ProgressStyle::with_template("{spinner:.green} {msg}")?.tick_strings(&[
    //         "◉◯◯◯◯◯◯",
    //         "◯◉◯◯◯◯◯",
    //         "◯◯◉◯◯◯◯",
    //         "◯◯◯◉◯◯◯",
    //         "◯◯◯◯◉◯◯",
    //         "◯◯◯◯◯◉◯",
    //         "◯◯◯◯◯◯◉",
    //         "◯◯◯◯◯◉◯",
    //         "◯◯◯◯◉◯◯",
    //         "◯◯◯◉◯◯◯",
    //         "◯◯◉◯◯◯◯",
    //         "◯◉◯◯◯◯◯",
    //         "== OK ==",
    //     ]),
    // );

    bar.set_style(
        ProgressStyle::with_template("{spinner:.white} {msg}")?.tick_strings(&[
            "🌑🌑🌑",
            "🌘🌑🌑",
            "🌗🌑🌑",
            "🌖🌑🌑",
            "🌕🌑🌑",
            "🌔🌑🌑",
            "🌓🌑🌑",
            "🌒🌑🌑",
            "🌑🌑🌑",
            "🌑🌘🌑",
            "🌑🌗🌑",
            "🌑🌖🌑",
            "🌑🌕🌑",
            "🌑🌔🌑",
            "🌑🌓🌑",
            "🌑🌒🌑",
            "🌑🌑🌑",
            "🌑🌑🌘",
            "🌑🌑🌗",
            "🌑🌑🌖",
            "🌑🌑🌕",
            "🌑🌑🌔",
            "🌑🌑🌓",
            "🌑🌑🌒",
            "🌑🌑🌑",
            "🌑🌑🌒",
            "🌑🌑🌓",
            "🌑🌑🌔",
            "🌑🌑🌕",
            "🌑🌑🌖",
            "🌑🌑🌗",
            "🌑🌑🌘",
            "🌑🌑🌑",
            "🌑🌒🌑",
            "🌑🌓🌑",
            "🌑🌔🌑",
            "🌑🌕🌑",
            "🌑🌖🌑",
            "🌑🌗🌑",
            "🌑🌘🌑",
            "🌑🌑🌑",
            "🌒🌑🌑",
            "🌓🌑🌑",
            "🌔🌑🌑",
            "🌕🌑🌑",
            "🌖🌑🌑",
            "🌗🌑🌑",
            "🌘🌑🌑",
            "✅ Done!  ", // Final state
        ]),
    );

    bar.set_message(message.to_string());
    bar.enable_steady_tick(Duration::from_millis(100));

    Ok(bar)
}
