use std::{io::Write, path::PathBuf};

use clap::Parser;

use crate::{
    cli::{address::Address, dump::DumpArgs},
    win::process::Process,
};

mod address;
mod dump;
mod output;

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
    Continue {
        #[arg(default_value_t = 1)]
        count: usize,
    },
    #[command(alias = "n")]
    Next {
        #[arg(default_value_t = 1)]
        count: usize,
    },
    #[command(alias = "s")]
    Step {
        #[arg(default_value_t = 1)]
        count: usize,
    },
    #[command(alias = "d")]
    Dump(DumpArgs),
    #[command(alias = "lm")]
    ListModules,
    #[command(alias = "ls")]
    ListSymbols,
    #[command(alias = "lt")]
    ListThreads,
}

pub fn run(args: Args) {
    let process = Process::new(args.path);
    let mut address = 0;

    let mut input = String::new();
    loop {
        print!("{:016X}> ", address);
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut input).unwrap();
        match Command::try_parse_from(std::iter::once("").chain(input.trim().split(' '))) {
            Ok(value) => match value {
                Command::Quit => {
                    break;
                }
                Command::Go { address: address_ } => {
                    if let Some(address_) = address_.to_raw(&process) {
                        address = address_;
                    }
                }
                Command::Break { address: _ } => {}
                Command::Continue { count: _ } => process.resume(),
                Command::Next { count: _ } => {}
                Command::Step { count: _ } => {}
                Command::Dump(args) => {
                    dump::run(&process, address, args);
                }
                Command::ListModules => {
                    for module in process.modules() {
                        println!(
                            "{}: {:016X} {:016X}",
                            module.name(),
                            module.base(),
                            module.size()
                        )
                    }
                }
                Command::ListSymbols => {
                    for module in process.modules() {
                        println!("{}:", module.name());
                        let Ok(symbols) = module.symbols() else {
                            continue;
                        };
                        for (symbol_name, symbol) in symbols {
                            println!("\t{}: {:016X}", symbol_name, symbol);
                        }
                    }
                }
                Command::ListThreads => {}
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
