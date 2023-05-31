#![feature(int_roundings)]

use clap::Parser;

use crate::cli::{run, Args};

mod cli;
mod dbg;

fn main() {
    run(Args::parse());
}
