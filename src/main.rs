#![allow(warnings)]

use clap::{Arg, ArgAction, Command};
use log::info;
use snapviewer::{
    geometry::TraceGeometry,
    load::{load_allocations, read_snap, read_snap_from_zip, SnapType},
    render_loop::RenderLoop,
};

fn cli() -> (SnapType, (u32, u32)) {
    let matches = Command::new("tomi: pyTOrch Memory Inspection tool")
        .arg(
            Arg::new("zip")
                .short('z')
                .long("zip")
                .help("Load snap from a .zip file")
                .action(ArgAction::Set)
                .num_args(1) // Exactly one path
                .value_name("ZIP_PATH")
                .conflicts_with("json"), // Cannot be used with --json
        )
        .arg(
            Arg::new("json")
                .short('j')
                .long("json")
                .help("Load snap from allocations.json and elements.json files")
                .action(ArgAction::Set)
                .num_args(2) // Exactly two paths
                .value_name("JSON_PATHS")
                .conflicts_with("zip"), // Cannot be used with --zip
        )
        .arg(
            Arg::new("res")
                .long("res")
                .help("Specify screen resolution as <WIDTH> <HEIGHT>")
                .action(ArgAction::Set)
                .num_args(2) // Exactly two u32 integers
                .value_names(["WIDTH", "HEIGHT"]) // Names for the two values
                .value_parser(clap::value_parser!(u32)) // Ensure values are parsed as u32
                .required(true), // Make the --res argument mandatory
        )
        .get_matches();

    let snap_type = if let Some(zip_paths) = matches.get_many::<String>("zip") {
        let path: Vec<_> = zip_paths.map(|s| s.as_str()).collect();
        SnapType::Zip {
            path: path[0].to_string(),
        }
    } else if let Some(json_paths) = matches.get_many::<String>("json") {
        let paths: Vec<_> = json_paths.map(|s| s.as_str()).collect();

        SnapType::Json {
            allocations_path: paths[0].to_string(),
            elements_path: paths[1].to_string(),
        }
    } else {
        eprintln!(
            "No valid snap arguments provided. Use --zip <PATH> or --json <ALLOC_PATH> <ELEM_PATH>."
        );
        std::process::exit(1);
    };

    // Since --res is now required, we can unwrap safely or use a direct get.
    // get_many will always return Some now because the argument is required.
    let res_values = matches.get_many::<u32>("res").unwrap();
    let values: Vec<u32> = res_values.copied().collect();
    let resolution = (values[0], values[1]);

    (snap_type, resolution)
}

fn app() -> anyhow::Result<()> {
    pretty_env_logger::formatted_timed_builder()
        .filter_level(log::LevelFilter::Off)
        .filter_module("snapviewer", log::LevelFilter::Info)
        .init();

    let (snap_type, resolution) = cli();

    let allocs = read_snap(snap_type)?;

    let render_loop = RenderLoop::from_allocations(allocs, resolution);

    render_loop.run();

    Ok(())
}

fn main() {
    if let Err(e) = app() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
    // else quit normally
}
