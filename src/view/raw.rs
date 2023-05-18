use eframe::egui::{Label, RichText, TextStyle, Ui};
use egui_extras::{Column, TableBuilder};

use crate::{AppState, AppView};

pub struct RawView {
    address: u64,
    data_offset: usize,
    data_length: usize,
}

impl RawView {
    pub fn new(address: u64, data_offset: usize, data_length: usize) -> Self {
        Self {
            address,
            data_offset,
            data_length,
        }
    }
}

impl AppView for RawView {
    fn title(&self) -> String {
        format!("Raw ({:016X})", self.address)
    }

    fn ui(&mut self, state: &mut AppState, ui: &mut Ui) {
        let data =
            &state.data[self.data_offset as usize..(self.data_offset + self.data_length) as usize];
        // render table
        let row_height = ui.text_style_height(&TextStyle::Monospace);
        TableBuilder::new(ui)
            .min_scrolled_height(0.0)
            .max_scroll_height(f32::INFINITY)
            .columns(Column::auto(), 2)
            .column(Column::remainder())
            .header(row_height, |mut row| {
                row.col(|_ui| {});
                row.col(|ui| {
                    ui.monospace("00 01 02 03 04 05 06 07 08 09 0A 0B 0C 0D 0E 0F");
                });
                row.col(|_ui| {});
            })
            .body(|body| {
                let aligned_address = self.address as usize / 16;
                let aligned_address_offset = self.address as usize % 16;
                // render rows
                body.rows(
                    row_height,
                    (data.len() + aligned_address_offset).div_ceil(16),
                    |index, mut row| {
                        let data = &data[if index == 0 {
                            0
                        } else {
                            index * 16 - aligned_address_offset
                        }
                            ..(index * 16 + 16 - aligned_address_offset).min(data.len())];
                        // render cols
                        row.col(|ui| {
                            ui.add(
                                Label::new(
                                    RichText::from(format!(
                                        "{:016X}",
                                        (index + aligned_address) * 16
                                    ))
                                    .monospace(),
                                )
                                .wrap(false),
                            );
                        });
                        row.col(|ui| {
                            let mut text = data
                                .iter()
                                .map(|&elem| format!("{:02X}", elem))
                                .collect::<Vec<_>>()
                                .join(" ");
                            if index == 0 {
                                text = format!(
                                    "{}{}",
                                    "   ".repeat(aligned_address_offset as usize),
                                    text
                                );
                            }
                            ui.add(Label::new(RichText::from(text).monospace()).wrap(false));
                        });
                        row.col(|ui| {
                            let mut text = data
                                .iter()
                                .map(|&elem| {
                                    if elem >= 0x20 && elem <= 0x7F {
                                        elem as char
                                    } else {
                                        '.'
                                    }
                                })
                                .collect::<String>();
                            if index == 0 {
                                text = format!(
                                    "{}{}",
                                    " ".repeat(aligned_address_offset as usize),
                                    text
                                );
                            }
                            ui.add(Label::new(RichText::from(text).monospace()).wrap(false));
                        });
                    },
                );
            });
    }
}
