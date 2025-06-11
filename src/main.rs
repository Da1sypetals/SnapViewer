use clap::{Arg, ArgAction, Command};
use snapviewer::{load::read_snap, render_loop::RenderLoop};

#[derive(Debug)]
pub struct CliArg {
    pub path: String,
    pub resolution: (u32, u32),
    pub log_level: log::LevelFilter,
}

pub fn cli() -> CliArg {
    let matches = Command::new("SnapViewer: PyTorch snapshot viewer")
        .arg(
            Arg::new("path")
                .short('p')
                .long("path")
                .help("Path of the .zip file to load snapshot from")
                .action(ArgAction::Set)
                .num_args(1)
                .value_name("PATH")
                .value_parser(clap::value_parser!(String))
                .required(true),
        )
        .arg(
            Arg::new("res")
                .long("res")
                .help("Specify screen resolution as <WIDTH> <HEIGHT>")
                .action(ArgAction::Set)
                .num_args(2)
                .value_names(["WIDTH", "HEIGHT"])
                .value_parser(clap::value_parser!(u32))
                .required(true),
        )
        .arg(
            Arg::new("log")
                .long("log")
                .help("Set the log level (info, trace). Default is error.")
                .value_name("LEVEL")
                .value_parser(["info", "trace"])
                .action(ArgAction::Set)
                .required(false),
        )
        .get_matches();

    let path = matches.get_one::<String>("path").unwrap().clone();

    let res_values = matches.get_many::<u32>("res").unwrap();
    let values: Vec<u32> = res_values.copied().collect();
    let resolution = (values[0], values[1]);

    let log_level = match matches.get_one::<String>("log").map(String::as_str) {
        Some("info") => log::LevelFilter::Info,
        Some("trace") => log::LevelFilter::Trace,
        _ => log::LevelFilter::Error,
    };

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
