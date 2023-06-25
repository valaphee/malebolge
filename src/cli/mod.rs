use std::{io::Write, path::PathBuf};

use clap::Parser;

use crate::{cli::address::Address, win::process::Process};

mod address;

#[derive(Parser)]
pub struct Args {
    path: PathBuf,
}

#[derive(Parser)]
pub enum Command {
    #[command(alias = "q")]
    Quit,
    #[command(alias = "g")]
    Go { address: Address },
    #[command(alias = "b")]
    Break { address: Address },
    #[command(alias = "c")]
    Continue { count: usize },
    #[command(alias = "n")]
    Next { count: usize },
    #[command(alias = "s")]
    Step { count: usize },
    #[command(alias = "lm")]
    ListModules,
    #[command(alias = "ls")]
    ListSymbols,
}

pub fn run(args: Args) {
    let process = Process::new(args.path);
    let mut current_address = 0;

    let mut input = String::new();
    loop {
        print!("{:016X}> ", current_address);
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut input).unwrap();
        match Command::try_parse_from(std::iter::once("").chain(input.trim().split(' '))) {
            Ok(value) => match value {
                Command::Quit => {
                    break;
                }
                Command::Go { address } => {
                    current_address = address.to_raw(&process).unwrap();
                }
                Command::Break { address: _ } => {}
                Command::Continue { count: _ } => {}
                Command::Next { count: _ } => {}
                Command::Step { count: _ } => {}
                Command::ListModules => {
                    for (module_name, module) in process.modules() {
                        println!("{}: {:016X} {:016X}", module_name, module.base(), module.size())
                    }
                }
                Command::ListSymbols => {
                    for (module_name, module) in process.modules() {
                        println!("{}:", module_name);
                        for (symbol_name, symbol) in module.symbols() {
                            println!("\t{}: {:016X}", symbol_name, symbol);
                        }
                    }
                }
            },
            Err(error) => {
                let _ = error.print();
            }
        }
        input.clear();
        println!();
        std::io::stdout().flush().unwrap();
    }
}
