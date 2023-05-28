use clap::Parser;

use crate::cli::{dump::DumpArgs, info::InfoArgs};

pub mod dump;
mod info;

#[derive(Parser)]
pub enum Command {
    Info(InfoArgs),
    Dump(DumpArgs),
}

pub fn run(command: Command) {
    match command {
        Command::Info(args) => info::run(args),
        Command::Dump(args) => dump::run(args),
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
