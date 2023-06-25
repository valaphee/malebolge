use std::{io::Write, path::PathBuf};

use clap::Parser;

use crate::{cli::address::Address};

mod address;

#[derive(Parser)]
pub struct Args {
    path: PathBuf,
}

#[derive(Parser)]
pub enum Command {
    Quit,
    Go { address: Address },
    Break { address: Address },
    Continue { count: usize },
    Next { count: usize },
    Step { count: usize },
}

pub fn run(args: Args) {
    let mut input = String::new();
    loop {
        print!("> ");
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut input).unwrap();
        match Command::try_parse_from(std::iter::once("").chain(input.trim().split(' '))) {
            Ok(value) => match value {
                Command::Quit => {
                    break;
                }
                Command::Break { address } => {}
                Command::Continue { count } => {}
                Command::Next { count } => {}
                Command::Step { count } => {}
                Command::Go { address } => {}
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

pub fn print_table(columns: Vec<String>, rows: Vec<Vec<String>>, box_drawing: bool) {
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

    if box_drawing {
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
