use crate::allocation::{Allocation, ElementData, RawAllocationData};
use crate::utils::{ALLOCATIONS_FILE_NAME, ELEMENTS_FILE_NAME};
use log::info;
use std::fs::File;
use std::io::Read;
use zip::ZipArchive;

#[derive(Debug)]
pub struct RawSnap {
    /// Path to snapshot pickle file
    pub(crate) path: String,
    pub(crate) allocations: String,
    pub(crate) elements: String,
}

pub fn read_snap(zip_file_path: &str) -> anyhow::Result<Vec<Allocation>> {
    info!("Loading snapshot...");
    let rawsnap = read_snap_from_zip(zip_file_path)?;
    load_allocations(rawsnap)
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
pub fn read_snap_from_zip(zip_file_path: &str) -> anyhow::Result<RawSnap> {
    info!("Loading json strings from zip...");

    let mut allocations: Option<String> = None;
    let mut elements: Option<String> = None;

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
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                allocations = Some(content);
            } else if filename == ELEMENTS_FILE_NAME {
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                elements = Some(content);
            }
        }
    }

    match (allocations, elements) {
        (None, None) => Err(anyhow::anyhow!("allocations and elements not found!")),
        (None, Some(_)) => Err(anyhow::anyhow!("allocations not found!")),
        (Some(_), None) => Err(anyhow::anyhow!("elements not found!")),
        (Some(allocs), Some(elems)) => Ok(RawSnap {
            path: zip_file_path.into(),
            allocations: allocs,
            elements: elems,
        }),
    }
}

pub fn load_allocations(rawsnap: RawSnap) -> anyhow::Result<Vec<Allocation>> {
    info!("Parsing json to data structure...");

    let raw_allocs: Vec<RawAllocationData> =
        serde_json::from_str(&rawsnap.allocations).map_err(|e| {
            anyhow::anyhow!(
                "Failed to parse allocations JSON from '{:?}': {}",
                rawsnap.path,
                e
            )
        })?;

    // elements.json is a list, where each item has a "frames" key.
    let elements_data: Vec<ElementData> = serde_json::from_str(&rawsnap.elements).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse elements JSON from '{:?}': {}",
            rawsnap.path,
            e
        )
    })?;

    // Check if the number of allocations matches the number of element data (callstacks)
    if raw_allocs.len() != elements_data.len() {
        return Err(anyhow::anyhow!(
            "Mismatch in the number of entries: {} allocations vs {} elements",
            raw_allocs.len(),
            elements_data.len()
        )
        .into());
    }

    // Combine the data from raw_allocs and elements_data (callstacks)
    let allocations: Vec<Allocation> = raw_allocs
        .into_iter()
        .zip(elements_data.into_iter())
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
