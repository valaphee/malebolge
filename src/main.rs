#![feature(try_blocks)]
#![feature(int_roundings)]

use clap::Parser;

use crate::cli::Args;

mod cli;
mod win;

fn main() {
    cli::run(Args::parse())
}
