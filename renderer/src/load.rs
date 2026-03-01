use crate::allocation::{Allocation, RawAllocationData};
use crate::constants::ALLOCATIONS_FILE_NAME;
use crate::utils::{get_spinner, memory_usage};
use indicatif::ProgressIterator;
use log::info;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;

/// Reads from dir.join(allocations.json) and deserialize
///
/// ## Returns
/// An atomic refcounted pointer to allocation slice.
///
/// Executed at start
pub fn read_allocations(dir: &Path) -> anyhow::Result<Arc<[Allocation]>> {
    info!("Loading json strings from zip...");

    // Open the zip file
    let allocations_path = dir.join(ALLOCATIONS_FILE_NAME);
    let mut file = File::open(allocations_path)?;

    info!("Reading {} to string", ALLOCATIONS_FILE_NAME);

    let bar = get_spinner(&format!("Reading {} to string", ALLOCATIONS_FILE_NAME))?;

    let mut content = String::new();
    file.read_to_string(&mut content)?;

    bar.finish();
    println!("Memory after loading allocs: {} MiB", memory_usage());

    let bar = get_spinner("Deserializing allocations...")?;

    let raw_allocs: Vec<RawAllocationData> = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse allocations JSON from '{:?}': {}", dir, e))?;
    println!("Memory after deserializing allocs: {} MiB", memory_usage());

    bar.finish();

    let allocations: Arc<[Allocation]> = raw_allocs
        .into_iter()
        .map(|raw_alloc| {
            let peak_base = *raw_alloc.offsets.iter().max().unwrap();
            let peak_timestamps = raw_alloc
                .timesteps
                .iter()
                .zip(raw_alloc.offsets.iter())
                .filter_map(|(&timestamp, &offset)| {
                    if offset == peak_base {
                        // if this timestep has peak memory
                        Some(timestamp)
                    } else {
                        None
                    }
                })
                .collect();

            let peak = peak_base + raw_alloc.size;
            Allocation {
                timesteps: raw_alloc.timesteps,
                offsets: raw_alloc.offsets,
                size: raw_alloc.size,
                peak_mem: peak,
                peak_timestamps,
            }
        })
        .progress()
        .collect();

    Ok(allocations)
}
