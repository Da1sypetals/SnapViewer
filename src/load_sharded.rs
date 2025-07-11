use crate::allocation::{Allocation, ElementData, RawAllocationData};
use crate::utils::{ALLOCATIONS_FILE_NAME, get_spinner, memory_usage};
use indicatif::ProgressIterator;
use log::info;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::sync::Arc;
use zip::ZipArchive;

/// Parses python-generated element file name.
/// Format:
/// def element_file_name(shard_idx: int):
///      return f"elements_{shard_idx}.json"
/// Returns i
pub fn element_file_name(filename: &str) -> Option<usize> {
    filename
        .strip_prefix("elements_")
        .and_then(|s| s.strip_suffix(".json"))
        .and_then(|shard_str| shard_str.parse::<usize>().ok())
}
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
pub fn read_snap_sharded(zip_file_path: &str) -> anyhow::Result<Arc<[Allocation]>> {
    info!("Loading json strings from zip...");

    let mut raw_allocs: Vec<RawAllocationData> = Vec::new();

    // Open the zip file
    let file = File::open(zip_file_path)?;

    // Create a ZipArchive from the file
    let mut archive = ZipArchive::new(file)?;

    let num_shard = archive
        .file_names() // Get an iterator over the names of all files in the archive.
        .filter_map(|name| {
            // Iterate over each file name, transforming and filtering it.
            let path = std::path::Path::new(name); // Convert the file name string into a Path for easier manipulation.
            if path.extension().and_then(|s| s.to_str()) == Some("meta") {
                // Check if the file's extension is "meta".
                // `and_then` is used to safely convert Option<OsStr> to Option<&str> and compare.
                path.file_name() // Get the final component of the path (the file name itself).
                    .and_then(|s| s.to_str()) // Convert OsStr to &str safely.
                    .and_then(|filename| filename.split('.').next()) // Split the filename by '.' and take the first part (e.g., "123" from "123.meta").
                    .and_then(|s| s.parse::<usize>().ok()) // Parse the extracted string part into a `usize` (unsigned integer), returning `Some(value)` on success, `None` on failure.
            } else {
                None // If the extension is not "meta", return None to filter this item out.
            }
        })
        .next() // Take the first `Some(usize)` value found by `filter_map`. If no matching file is found, this will be `None`.
        .ok_or_else(|| anyhow::anyhow!("Shard count of elements.json is not found!"))?;
    // If `next()` returned `None` (meaning no .meta file with a parseable shard count was found),
    // report and propagate error.

    let mut elements_shards: BTreeMap<usize, Vec<ElementData>> = BTreeMap::new();

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
            } else if let Some(shard_idx) = element_file_name(filename) {
                info!("Reading elements shard {} to string", shard_idx);
                let bar = get_spinner(&format!("Reading elements shard {} to string", shard_idx))?;

                let mut content = String::new();
                file.read_to_string(&mut content)?;

                bar.finish();
                println!(
                    "Memory after loading elems shard {}: {} MiB",
                    shard_idx,
                    memory_usage()
                );

                let bar = get_spinner("Deserializing elements...")?;
                let elements_shard: Vec<ElementData> =
                    serde_json::from_str(&content).map_err(|e| {
                        anyhow::anyhow!(
                            "Failed to parse elements JSON from '{:?}': {}",
                            zip_file_path,
                            e
                        )
                    })?;

                elements_shards.insert(shard_idx, elements_shard);

                println!(
                    "Memory after deserializing elements shard {}: {} MiB",
                    shard_idx,
                    memory_usage()
                );
                bar.finish();
            } else {
                println!("Unrecognized file: {}", filename);
            }
        }
    }

    if !(0..num_shard).all(|i| elements_shards.contains_key(&i)) {
        return Err(anyhow::anyhow!(
            "# of shards mismatch with metadata: total {} shards",
            num_shard
        ));
    }

    let num_elem: usize = elements_shards.values().map(|x| x.len()).sum();
    if raw_allocs.len() != num_elem || raw_allocs.is_empty() {
        return Err(anyhow::anyhow!(
            "Mismatch in the number of entries (required non-empty equal): {} allocations vs {} elements",
            raw_allocs.len(),
            num_elem
        ));
    }

    let elements_iterator = elements_shards
        // values are sorted by key
        .into_iter()
        // Flatten the Option<Vec<T>> to an Iterator<Item = &T>
        .flat_map(|(_, outer_option)| outer_option);

    let allocations: Arc<[Allocation]> = raw_allocs
        .into_iter()
        // flat map does not have exact size
        // this is guaranteed by the checks above, but rustc does not know it
        .progress()
        .zip(elements_iterator)
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
        .collect();

    Ok(allocations)
}
