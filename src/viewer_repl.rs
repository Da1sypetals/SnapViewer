use rustyline::{DefaultEditor, error::ReadlineError};
use std::sync::mpsc::{self};

pub const HELP_MESSAGE: &str = r#"
Available commands:
    --show <alloc_idx>  : show the mesh with the given index
    --help              : show this help message
    --quit              : quit viewer
"#;

pub struct CommunicateSender {
    pub alloc_idx: mpsc::Sender<isize>,
    pub terminate: mpsc::Sender<()>,
}

pub struct CommunicateReceiver {
    pub alloc_idx: mpsc::Receiver<isize>,
    pub terminate: mpsc::Receiver<()>,
}

pub fn make_communicate() -> (CommunicateSender, CommunicateReceiver) {
    let (alloc_sender, alloc_receiver) = mpsc::channel();
    let (terminate_sender, terminate_receiver) = mpsc::channel();

    let sender = CommunicateSender {
        alloc_idx: alloc_sender,
        terminate: terminate_sender,
    };
    let receiver = CommunicateReceiver {
        alloc_idx: alloc_receiver,
        terminate: terminate_receiver,
    };

    (sender, receiver)
}

pub struct Repl {
    pub sender: CommunicateSender,
}
impl Repl {
    pub fn show(&self, args: &[&str]) -> anyhow::Result<()> {
        if args.len() != 1 {
            return Err(anyhow::anyhow!("Usage: --show <alloc_idx>"));
        }

        let alloc_idx = args[0].parse()?;
        self.sender.alloc_idx.send(alloc_idx)?;

        Ok(())
    }
}

impl Repl {
    pub fn new(sender: CommunicateSender) -> Self {
        Repl { sender }
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let mut rl = DefaultEditor::new()?;

        loop {
            let readline = rl.readline("viewer> ");
            match readline {
                // read line success
                Ok(line) => {
                    rl.add_history_entry(line.as_str())?;
                    let command = line.trim();
                    if command.len() == 0 {
                        continue;
                    }
                    if command.starts_with("--") {
                        // special command
                        let argv = command.split_whitespace().collect::<Vec<_>>();

                        match argv[0] {
                            "--show" => {
                                // communicate to renderer: update selected mesh
                                match self.show(&argv[1..]) {
                                    Ok(_) => {}
                                    Err(e) => println!("Error: {}", e),
                                }
                            }
                            "--help" => {
                                println!("{}", HELP_MESSAGE);
                            }
                            "--quit" => {
                                println!("👋 Bye!");
                                self.sender.terminate.send(())?;
                                break;
                            }
                            _ => {
                                //
                                println!("Unknown command: {}", command);
                            }
                        }
                    } else {
                        // normal command, echo for now
                        println!("Unknown command: {}", command)
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    println!("👋 Bye!");
                    self.sender.terminate.send(())?;
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
}
