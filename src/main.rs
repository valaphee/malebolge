use clap::Parser;
use crate::cli::{Command, run};

mod cli;

fn main() {
    run(Command::parse());
}
