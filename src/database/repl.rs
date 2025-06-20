use clap::{Arg, ArgAction, Command};
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use snapviewer::allocation::Allocation;
use snapviewer::database::sqlite::AllocationDatabase;
use snapviewer::load::read_snap;

#[derive(Debug)]
pub struct CliArg {
    pub path: String,
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

    let log_level = match matches.get_one::<String>("log").map(String::as_str) {
        Some("info") => log::LevelFilter::Info,
        Some("trace") => log::LevelFilter::Trace,
        _ => log::LevelFilter::Error,
    };

    CliArg { path, log_level }
}

pub fn repl(allocations: &[Allocation]) -> anyhow::Result<()> {
    let db = AllocationDatabase::from_allocations(allocations)?;

    // `()` can be used when no completer is required
    let mut rl = DefaultEditor::new()?;

    loop {
        let readline = rl.readline("sql> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                let command = line.trim();
                if command.len() == 0 {
                    continue;
                }
                if command == "quit" {
                    println!("👋 Bye!");
                    break;
                }
                match db.execute(command) {
                    Ok(output) => {
                        // rustfmt do not collapse
                        println!("✅ SQL execution OK");
                        println!("{}", output);
                    }
                    Err(e) => {
                        println!("⚠️ SQL execution Error");
                        println!("{}", e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("👋 Bye!");
                break;
            }
            Err(err) => {
                println!("Internal rustyline error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}

fn app() -> anyhow::Result<()> {
    let args = cli();

    pretty_env_logger::formatted_timed_builder()
        .filter_level(log::LevelFilter::Off)
        .filter_module("snapviewer", args.log_level)
        .init();

    let allocs = read_snap(&args.path)?;

    repl(&allocs)?;

    Ok(())
}

fn main() {
    if let Err(e) = app() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
    // else quit normally
}
