use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::cli::{dbg::DbgArgs, dump::DumpArgs};

mod dbg;
mod dump;
mod info;

#[derive(Parser)]
pub struct Args {
    path: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Dbg(DbgArgs),
    Info,
    Dump(DumpArgs),
}

pub fn run(args: Args) {
    match args.command {
        Command::Dbg(cmd_args) => dbg::run(args.path, cmd_args),
        Command::Info => info::run(args.path),
        Command::Dump(cmd_args) => dump::run(args.path, cmd_args),
    }
}

pub fn print_table(columns: Vec<String>, rows: Vec<Vec<String>>, pretty: bool) {
    let mut column_widths = columns
        .iter()
        .map(|column| column.len())
        .collect::<Vec<_>>();
    for row in rows.iter() {
        for (column_index, cell) in row.iter().enumerate() {
            let cell_width = cell.len();
            if column_widths[column_index] < cell_width {
                column_widths[column_index] = cell_width;
            }
        }
    }

    if pretty {
        println!(
            "╔{}╗",
            columns
                .iter()
                .enumerate()
                .map(|(column_index, column)| format!(
                    "{:═^1$}",
                    format!(" {} ", column),
                    column_widths[column_index] + 2
                ))
                .collect::<Vec<_>>()
                .join("╤")
        );
        for row in rows {
            println!(
                "║ {} ║",
                row.iter()
                    .enumerate()
                    .map(|(column_index, cell)| format!(
                        "{:<1$}",
                        cell, column_widths[column_index]
                    ))
                    .collect::<Vec<_>>()
                    .join(" │ ")
            );
        }
        println!(
            "╚{}╝",
            columns
                .iter()
                .enumerate()
                .map(|(column_index, column)| format!(
                    "{:═^1$}",
                    format!(" {} ", column),
                    column_widths[column_index] + 2
                ))
                .collect::<Vec<_>>()
                .join("╧")
        );
    } else {
        println!(
            "{}",
            columns
                .iter()
                .enumerate()
                .map(|(column_index, column)| format!(
                    "{:<1$}",
                    format!("{}", column),
                    column_widths[column_index]
                ))
                .collect::<Vec<_>>()
                .join(" ")
        );
        for row in rows {
            println!(
                "{}",
                row.iter()
                    .enumerate()
                    .map(|(column_index, cell)| {
                        format!("{:<1$}", cell, column_widths[column_index])
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
    }
}
