use crate::allocation::{Allocation, ElementData, RawAllocationData};
use crate::utils::{ALLOCATIONS_FILE_NAME, ELEMENTS_FILE_NAME, get_spinner, memory_usage};
use indicatif::ProgressIterator;
use log::info;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use zip::ZipArchive;

/// Unzips "allocations.json" and "elements.json" from a zip file into memory.
///
/// ## Arguments
/// * `zip_file_path` - The path to the zip file.
///
/// ## Returns
/// A `Result` containing a tuple of `(Option<String>, Option<String>)` where the first
/// `String` is the content of "allocations.json" and the second is the content of
/// "elements.json", or an `io::Error` if an error occurs.
///
/// Executed at start
pub fn read_snap(zip_file_path: &str) -> anyhow::Result<Arc<[Allocation]>> {
    info!("Loading json strings from zip...");

    let mut raw_allocs: Vec<RawAllocationData> = Vec::new();
    let mut elements: Vec<ElementData> = Vec::new();

    // Open the zip file
    let file = File::open(zip_file_path)?;

    // Create a ZipArchive from the file
    let mut archive = ZipArchive::new(file)?;

    // Iterate over each file in the zip archive
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue, // Skip if no valid name
        };

        if outpath.extension().and_then(|s| s.to_str()) == Some("json") {
            let filename = outpath.file_name().and_then(|s| s.to_str()).unwrap_or("");

            if filename == ALLOCATIONS_FILE_NAME {
                info!("Reading {} to string", ALLOCATIONS_FILE_NAME);
                let bar = get_spinner(&format!("Reading {} to string", ALLOCATIONS_FILE_NAME))?;

                let mut content = String::new();
                file.read_to_string(&mut content)?;

                bar.finish();
                println!("Memory after loading allocs: {} MiB", memory_usage());

                let bar = get_spinner("Deserializing allocations...")?;

                raw_allocs = serde_json::from_str(&content).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to parse allocations JSON from '{:?}': {}",
                        zip_file_path,
                        e
                    )
                })?;
                println!("Memory after deserializing allocs: {} MiB", memory_usage());

                bar.finish();
            } else if filename == ELEMENTS_FILE_NAME {
                info!("Reading {} to string", ELEMENTS_FILE_NAME);
                let bar = get_spinner(&format!("Reading {} to string", ELEMENTS_FILE_NAME))?;

                let mut content = String::new();
                file.read_to_string(&mut content)?;

                bar.finish();
                println!("Memory after loading elems: {} MiB", memory_usage());

                let bar = get_spinner("Deserializing elements...")?;
                elements = serde_json::from_str(&content).map_err(|e| {
                    anyhow::anyhow!(
                        "Failed to parse elements JSON from '{:?}': {}",
                        zip_file_path,
                        e
                    )
                })?;

                println!(
                    "Memory after deserializing elements: {} MiB",
                    memory_usage()
                );
                bar.finish();
            }
        }
    }

    if raw_allocs.len() != elements.len() || raw_allocs.is_empty() {
        return Err(anyhow::anyhow!(
            "Mismatch in the number of entries (required non-empty equal): {} allocations vs {} elements",
            raw_allocs.len(),
            elements.len()
        ));
    }

    let allocations: Arc<[Allocation]> = raw_allocs
        .into_iter()
        .zip(elements)
        .map(|(raw_alloc, element_data)| {
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
                callstack: element_data.frames, // element_data.frames is Vec<Frame>
                peak_mem: peak,
                peak_timestamps,
            }
        })
        .progress()
        .collect();

    Ok(allocations)
}
