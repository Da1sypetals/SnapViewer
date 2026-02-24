use crate::constants::UNITS;
use indicatif::{ProgressBar, ProgressStyle};
use memory_stats::memory_stats;
use std::time::Duration;

#[cfg(feature = "python")]
use pyo3::{PyErr, exceptions::PyRuntimeError};

#[cfg(feature = "python")]
pub trait IntoPyErr {
    fn into_py_runtime_err(self) -> PyErr;
}

#[cfg(feature = "python")]
impl IntoPyErr for anyhow::Error {
    fn into_py_runtime_err(self) -> PyErr {
        PyRuntimeError::new_err(self.to_string())
    }
}

pub fn memory_usage() -> f64 {
    memory_stats().unwrap().virtual_mem as f64 / (1024.0 * 1024.0)
}

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

pub fn get_spinner(message: &str) -> anyhow::Result<ProgressBar> {
    let bar = ProgressBar::new_spinner();

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
