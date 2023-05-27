use clap::Parser;

use crate::cli::{run, Command};

mod cli;

fn main() {
    run(Command::parse());
}
