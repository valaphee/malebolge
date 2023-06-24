use clap::Parser;

use crate::cli::Args;

mod cli;
mod ctx;

fn main() {
    cli::run(Args::parse())
}
