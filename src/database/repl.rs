use clap::{Arg, ArgAction, Command};
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;
use snapviewer::allocation::Allocation;
use snapviewer::database::sqlite::{AllocationDatabase, CREATE_SQL};
use snapviewer::load::read_snap;

pub const HELP_MSG: &str = "
🛢️ Execute any SqLite commands.
✨ Special commands:
      --help: display this help message
      --schema: display database schema of the memory snapshot
      --quit: exit SqLite REPL
";

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

fn print_schema() {
    println!("\n📊 Table schema:\n\n{}\n", CREATE_SQL);
}

fn print_help() {
    println!("{}", HELP_MSG);
}

pub fn repl(allocations: &[Allocation]) -> anyhow::Result<()> {
    let db = AllocationDatabase::from_allocations(allocations)?;

    // `()` can be used when no completer is required
    let mut rl = DefaultEditor::new()?;

    print_schema();
    print_help();

    loop {
        let readline = rl.readline("sql> ");
        match readline {
            // read line success
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                let command = line.trim();
                if command.len() == 0 {
                    continue;
                }

                // determine: special command or SQL command
                if command.starts_with("--") {
                    // is a special command
                    match command {
                        "--quit" => {
                            println!("👋 Bye!");
                            break;
                        }
                        "--help" => {
                            print_help();
                        }
                        "--schema" => {
                            print_schema();
                        }
                        _ => {
                            println!("Unexpected special command: {}", command);
                        }
                    }
                } else {
                    // is a SQL command
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
            }
            Err(ReadlineError::Interrupted) => {
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("👋 Bye!");
                break;
            }
            // read line error
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
