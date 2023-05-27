use clap::Parser;
use crate::cli::view::ViewArgs;

pub mod view;

#[derive(Parser)]
pub enum Command {
    View(ViewArgs)
}

pub fn run(command: Command) {
    match command {
        Command::View(args) => view::run(args),
    }
}
