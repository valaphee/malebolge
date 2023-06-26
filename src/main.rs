#![feature(try_blocks)]

use clap::Parser;

use crate::cli::Args;

mod cli;
mod win;

fn main() {
    cli::run(Args::parse())
}
