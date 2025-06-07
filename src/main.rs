#![allow(warnings)]

use clap::{Arg, ArgAction, Command};
use log::info;
use snapviewer::{
    geometry::TraceGeometry,
    load::{load_allocations, read_snap, read_snap_from_zip},
    render_loop::RenderLoop,
};
// Define the structure to hold all parsed command-line arguments
#[derive(Debug)]
pub struct CliArg {
    pub path: String,
    pub resolution: (u32, u32),
    pub log_level: log::LevelFilter, // Added the log_info field
}

pub fn cli() -> CliArg {
    let matches = Command::new("SnapViewer: PyTorch snapshot viewer")
        .arg(
            Arg::new("path")
                .short('p')
                .long("path")
                .help("Path of the .zip file to load snapshot from")
                .action(ArgAction::Set) // Action to set the value
                .num_args(1) // Exactly one path
                .value_name("PATH")
                .value_parser(clap::value_parser!(String)) // Parse value as String
                .required(true),
        )
        .arg(
            Arg::new("res")
                .long("res")
                .help("Specify screen resolution as <WIDTH> <HEIGHT>")
                .action(ArgAction::Set) // Action to set the value
                .num_args(2) // Exactly two u32 integers
                .value_names(["WIDTH", "HEIGHT"]) // Names for the two values
                .value_parser(clap::value_parser!(u32)) // Ensure values are parsed as u32
                .required(true), // Make the --res argument mandatory
        )
        .arg(
            Arg::new("log-info") // New argument for log_info
                .long("log-info")
                .help("Enable logging of additional information")
                .action(ArgAction::SetTrue), // This action sets the argument to true if present
        )
        .get_matches();

    // Path is required, so we can safely unwrap the value.
    let path = matches.get_one::<String>("path").unwrap().clone();

    // Since --res is required, we can safely unwrap the values.
    // get_many will always return Some now because the argument is required.
    let res_values = matches.get_many::<u32>("res").unwrap();
    let values: Vec<u32> = res_values.copied().collect();
    let resolution = (values[0], values[1]);

    // Check if the --log-info flag was present
    let log_info = matches.get_flag("log-info"); // get_flag returns true if the flag was present

    let log_level = if log_info {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Error
    };

    // Return the CliArg struct
    CliArg {
        path,
        resolution,
        log_level,
    }
}

fn app() -> anyhow::Result<()> {
    let args = cli();

    pretty_env_logger::formatted_timed_builder()
        .filter_level(log::LevelFilter::Off)
        .filter_module("snapviewer", args.log_level)
        .init();

    let allocs = read_snap(&args.path)?;

    let render_loop = RenderLoop::from_allocations(allocs, args.resolution);

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
