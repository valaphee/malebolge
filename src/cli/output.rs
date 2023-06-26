use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref ANSI_REGEX: Regex =
        Regex::new(r"(?:\x1B[@-Z\\-_]|[\x80-\x9A\x9C-\x9F]|(?:\x1B\[|\x9B)[0-?]*[ -/]*[@-~])")
            .unwrap();
}

pub enum Output {
    Normal,
    Pretty,
}

impl Output {
    pub fn print_table(&self, columns: Vec<String>, rows: Vec<Vec<String>>) {
        let mut column_widths = columns
            .iter()
            .map(|column| ANSI_REGEX.replace_all(column, "").len())
            .collect::<Vec<_>>();
        for row in rows.iter() {
            for (column_index, cell) in row.iter().enumerate() {
                let cell_width = ANSI_REGEX.replace_all(cell, "").len();
                if column_widths[column_index] < cell_width {
                    column_widths[column_index] = cell_width;
                }
            }
        }

        match self {
            Output::Normal => {
                println!(
                    "{}",
                    columns
                        .iter()
                        .enumerate()
                        .map(|(column_index, column)| format!(
                            "{:<1$}",
                            format!("{}", column),
                            column_widths[column_index] + column.len()
                                - ANSI_REGEX.replace_all(column, "").len()
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
                                format!(
                                    "{:<1$}",
                                    cell,
                                    column_widths[column_index] + cell.len()
                                        - ANSI_REGEX.replace_all(cell, "").len()
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                    );
                }
            }
            Output::Pretty => {
                println!(
                    "╔{}╗",
                    columns
                        .iter()
                        .enumerate()
                        .map(|(column_index, column)| format!(
                            "{:═^1$}",
                            format!(" {} ", column),
                            column_widths[column_index] + 2 + column.len()
                                - ANSI_REGEX.replace_all(column, "").len()
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
                                cell,
                                column_widths[column_index] + cell.len()
                                    - ANSI_REGEX.replace_all(cell, "").len()
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
                            column_widths[column_index] + 2 + column.len()
                                - ANSI_REGEX.replace_all(column, "").len()
                        ))
                        .collect::<Vec<_>>()
                        .join("╧")
                );
            }
        }
    }
}
