use std::sync::mpsc::{self};

use rustyline::{DefaultEditor, error::ReadlineError};

pub struct CommunicateSender {
    pub sender: mpsc::Sender<usize>,
}

pub struct CommunicateReceiver {
    pub receiver: mpsc::Receiver<usize>,
}

pub fn make_communicate() -> (CommunicateSender, CommunicateReceiver) {
    let (sender, receiver) = mpsc::channel();

    let sender = CommunicateSender { sender };
    let receiver = CommunicateReceiver { receiver };

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
        self.sender.sender.send(alloc_idx)?;

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
            let readline = rl.readline("sql> ");
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
                            _ => {
                                //
                                println!("Unknown command: {}", command);
                            }
                        }
                    } else {
                        // normal command, echo for now
                        println!("echo: {}", command)
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
}
