use crate::allocation::Frame;
use ahash::{AHasher, RandomState};
use rand::Rng;
use std::cell::LazyCell;
use std::collections::HashMap;
use std::hash::Hash;
use three_d::Srgba;

pub static UNITS: [&str; 8] = ["", "Ki", "Mi", "Gi", "Ti", "Pi", "Ei", "Zi"];

pub fn format_bytes(bytes: u64) -> String {
    let mut num = bytes as f64;

    for unit in UNITS {
        if num.abs() < 1024.0 {
            return format!("{:.2} {}B", num, unit);
        }
        num /= 1024.0;
    }

    format!("{:.1}YiB", num) // Should be unreachable for typical u64 values
}

/// Rust has no default parameter value ...
pub fn format_bytes_precision(bytes: u64, precision: usize) -> String {
    let mut num = bytes as f64;

    for unit in UNITS {
        if num.abs() < 1024.0 {
            return format!("{:.2$} {}B", num, unit, precision);
        }
        num /= 1024.0;
    }

    format!("{:.1}YiB", num) // Should be unreachable for typical u64 values
}

pub fn sample_colors(n: usize) -> Vec<Srgba> {
    let mut rng = rand::rng();
    let mut colors = Vec::with_capacity(n);

    for _ in 0..n {
        let r = rng.random_range(0..=255);
        let g = rng.random_range(0..=255);
        let b = rng.random_range(0..=255);

        colors.push(Srgba::new(r, g, b, 30));
    }

    colors
}

/// (color, indices using that color)
pub fn sample_callstack_colors<'a>(
    stack: impl Iterator<Item = &'a Vec<Frame>>,
) -> Vec<(Srgba, Vec<usize>)> {
    let mut rng = rand::rng();
    let frames2index = map_iter_to_index(stack.map(|frames| {
        frames
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join("")
    }));

    frames2index
        .into_iter()
        .map(|(_, idxs)| {
            let r = rng.random_range(0..=255);
            let g = rng.random_range(0..=255);
            let b = rng.random_range(0..=255);

            (Srgba::new(r, g, b, 30), idxs)
        })
        .collect()
}

/// Maps an iterator of elements to a HashMap, where each element is mapped to its
/// number of occurrences in the iterated sequence.
///
/// # Arguments
///
/// * `iter` - An iterator that yields elements that implement the `Eq` and `Hash` traits.
///            The elements are taken by value to be owned by the HashMap.
///
/// # Returns
///
/// A `HashMap` where keys are the elements from the input iterator and values
/// are their respective counts.
pub fn map_iter_to_index<T, I>(iter: I) -> HashMap<T, Vec<usize>>
where
    I: Iterator<Item = T>,
    T: Eq + Hash,
{
    let mut counts = HashMap::new();

    for (i, elem) in iter.enumerate() {
        counts.entry(elem).or_insert(Vec::new()).push(i);
    }

    counts
}

#[cfg(test)]
mod tests {
    use crate::load::read_snap;

    use super::*;

    #[test]
    fn test_empty_iterator() {
        let data: Vec<i32> = Vec::new();
        let counts = map_iter_to_index(data.into_iter());
        assert!(counts.is_empty());
    }

    #[test]
    fn test_single_element_iterator() {
        let data = vec!["hello"];
        let counts = map_iter_to_index(data.into_iter());
        assert_eq!(counts.len(), 1);
        assert_eq!(counts.get("hello").unwrap().len(), 1);
    }

    #[test]
    fn test_multiple_unique_elements_iterator() {
        let data = vec![1, 2, 3, 4, 5];
        let counts = map_iter_to_index(data.into_iter());
        assert_eq!(counts.len(), 5);
        assert_eq!(counts.get(&1).unwrap().len(), 1);
        assert_eq!(counts.get(&2).unwrap().len(), 1);
        assert_eq!(counts.get(&3).unwrap().len(), 1);
        assert_eq!(counts.get(&4).unwrap().len(), 1);
        assert_eq!(counts.get(&5).unwrap().len(), 1);
    }

    #[test]
    fn test_elements_with_duplicates_iterator() {
        let data = vec![
            "apple", "banana", "apple", "orange", "banana", "apple", "grape",
        ];
        let counts = map_iter_to_index(data.into_iter());
        assert_eq!(counts.len(), 4);
        assert_eq!(counts.get("apple").unwrap().len(), 3);
        assert_eq!(counts.get("banana").unwrap().len(), 2);
        assert_eq!(counts.get("orange").unwrap().len(), 1);
        assert_eq!(counts.get("grape").unwrap().len(), 1);
    }

    #[test]
    fn test_integers_with_duplicates_iterator() {
        let data = vec![1, 2, 2, 3, 3, 3, 1, 4];
        let counts = map_iter_to_index(data.into_iter());
        assert_eq!(counts.len(), 4);
        assert_eq!(counts.get(&1).unwrap().len(), 2);
        assert_eq!(counts.get(&2).unwrap().len(), 2);
        assert_eq!(counts.get(&3).unwrap().len(), 3);
        assert_eq!(counts.get(&4).unwrap().len(), 1);
    }

    #[test]
    fn test_from_array_iterator() {
        let data = ["a", "b", "a", "c"];
        let counts = map_iter_to_index(data.iter().cloned());
        assert_eq!(counts.len(), 3);
        assert_eq!(counts.get("a").unwrap().len(), 2);
        assert_eq!(counts.get("b").unwrap().len(), 1);
        assert_eq!(counts.get("c").unwrap().len(), 1);
    }

    #[test]
    fn test_from_range_iterator() {
        let counts = map_iter_to_index((1..=5).flat_map(|x| std::iter::repeat(x).take(x)));
        assert_eq!(counts.len(), 5);
        assert_eq!(counts.get(&1).unwrap().len(), 1);
        assert_eq!(counts.get(&2).unwrap().len(), 2);
        assert_eq!(counts.get(&3).unwrap().len(), 3);
        assert_eq!(counts.get(&4).unwrap().len(), 4);
        assert_eq!(counts.get(&5).unwrap().len(), 5);
    }

    fn print_hashmap_formatted<K, V>(map: &HashMap<K, V>)
    where
        K: std::fmt::Debug + AsRef<str>, // K needs Debug for general printing and AsRef<str> for string slicing
        V: std::fmt::Debug,
    {
        for (key, value) in map {
            let key_str = key.as_ref();
            let formatted_key = if key_str.len() > 10 {
                &key_str[..10]
            } else {
                key_str
            };
            println!("Key: \"{}\", Value: {:?}", formatted_key, value);
        }
    }

    #[test]
    fn test_load_dump() {
        pretty_env_logger::formatted_timed_builder()
            .filter_level(log::LevelFilter::Off)
            .filter_module("snapviewer", log::LevelFilter::Info)
            .init();

        let allocs = read_snap(crate::load::SnapType::Zip {
            path: "./snap/small.zip".to_string(),
        })
        .unwrap();

        // map alloc to its concatenated callstack string
        let callstacks = allocs.iter().map(|a| {
            a.callstack
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join("")
        });

        let count = map_iter_to_index(callstacks);

        print_hashmap_formatted(&count);
    }
}
