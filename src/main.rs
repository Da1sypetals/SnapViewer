#![allow(warnings)]

use clap::{Arg, ArgAction, ArgGroup, Command};
use log::info;
use nalgebra::Vector2;
use snapviewer::{
    geometry::{AllocationGeometry, TraceGeometry},
    load::{load_allocations, read_snap_from_jsons, read_snap_from_zip},
    render_data::{RenderData, Transform},
    render_loop::render_loop,
    ui::{TranslateDir, WindowTransform},
};
use three_d::{
    degrees, vec2, vec3, Camera, Circle, ClearState, ColorMaterial, Event, FrameOutput, Geometry,
    Gm, Line, Mesh, MouseButton, Rectangle, Srgba, Viewport, Window, WindowSettings,
};

enum CliArg {
    Json { alloc: String, elem: String },
    Zip { path: String },
}

fn cli() -> CliArg {
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
        // You could also use an ArgGroup for mutual exclusivity, but conflicts_with is more direct here.
        // If you had more complex "either/or" scenarios, ArgGroup would be powerful.
        .get_matches();

    if let Some(zip_paths) = matches.get_many::<String>("zip") {
        let path: Vec<_> = zip_paths.map(|s| s.as_str()).collect();
        CliArg::Zip {
            path: path[0].to_string(),
        }
    } else if let Some(json_paths) = matches.get_many::<String>("json") {
        let paths: Vec<_> = json_paths.map(|s| s.as_str()).collect();

        CliArg::Json {
            alloc: paths[0].to_string(),
            elem: paths[1].to_string(),
        }
    } else {
        eprintln!(
            "No valid arguments provided. Use --zip <PATH> or --json <ALLOC_PATH> <ELEM_PATH>."
        );
        std::process::exit(1);
    }
}

pub fn load_geom(resolution: (u32, u32)) -> TraceGeometry {
    info!("Reading snapshot from disk...");
    // let rawsnap = read_snap_from_zip("snap/transformer.zip").unwrap();
    let rawsnap = read_snap_from_zip("snap/small.zip").unwrap();
    // let rawsnap = read_snap_from_jsons(
    //     "/home/da1sypetals/dev/torch-snapshot/snapshots/allocations.json",
    //     "/home/da1sypetals/dev/torch-snapshot/snapshots/elements.json",
    // )
    // .unwrap();

    info!("Loading allocations from zip...");
    let allocs = load_allocations(rawsnap).unwrap();

    TraceGeometry::from_allocations(&allocs, resolution)
}

fn main() {
    render_loop(load_geom);
}
