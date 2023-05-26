use std::ops::Range;

use eframe::egui::{Label, RichText, TextStyle, Ui};
use egui_extras::{Column, TableBuilder};

use crate::{
    gui::{AppContext, AppView},
    project::DataView,
};

pub struct RawView {
    rva: u64,
    data_range: Range<usize>,
}

impl RawView {
    pub fn new(rva: u64, data_view: DataView) -> Self {
        Self {
            rva,
            data_range: data_view.range,
        }
    }
}

impl AppView for RawView {
    fn title(&self) -> String {
        format!("Raw ({:016X})", self.rva)
    }

    fn ui(&mut self, context: &mut AppContext, ui: &mut Ui) {
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
                let data = &context.project.data()[self.data_range.clone()];
                let aligned_rva = self.rva as usize / 16;
                let aligned_rva_offset = self.rva as usize % 16;
                body.rows(
                    row_height,
                    (data.len() + aligned_rva_offset).div_ceil(16),
                    |index, mut row| {
                        let data = &data[if index == 0 {
                            0
                        } else {
                            index * 16 - aligned_rva_offset
                        }
                            ..(index * 16 + 16 - aligned_rva_offset).min(data.len())];

                        // address column
                        row.col(|ui| {
                            ui.add(
                                Label::new(
                                    RichText::from(format!("{:016X}", (index + aligned_rva) * 16))
                                        .monospace(),
                                )
                                .wrap(false),
                            );
                        });

                        // bytes column
                        row.col(|ui| {
                            let mut text = data
                                .iter()
                                .map(|&elem| format!("{:02X}", elem))
                                .collect::<Vec<_>>()
                                .join(" ");
                            if index == 0 {
                                text = format!(
                                    "{}{}",
                                    "   ".repeat(aligned_rva_offset as usize),
                                    text
                                );
                            }
                            ui.add(Label::new(RichText::from(text).monospace()).wrap(false));
                        });

                        // ascii column
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
                                text =
                                    format!("{}{}", " ".repeat(aligned_rva_offset as usize), text);
                            }
                            ui.add(Label::new(RichText::from(text).monospace()).wrap(false));
                        });
                    },
                );
            });
    }
}
