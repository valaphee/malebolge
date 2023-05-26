use std::{collections::HashSet, ops::Range};

use eframe::{
    egui::{
        text::LayoutJob, Align, Color32, FontId, Label, RichText, Sense, Style, TextStyle, Ui, Vec2,
    },
    epaint::text::TextWrapping,
};
use egui_extras::{Column, TableBuilder};
use iced_x86::{Decoder, DecoderOptions, Formatter, FormatterTextKind, NasmFormatter};

use crate::{
    gui::{AppContext, AppView},
    project::DataView,
};

pub struct AssemblyView {
    rva: u64,
    data_range: Range<usize>,

    last_rva: u64,
    rvas: HashSet<u64>,
    rows: Vec<Row>,
    scroll_to_row: Option<usize>,
}

impl AssemblyView {
    pub fn new(address: u64, data_view: DataView) -> Self {
        Self {
            rva: address,
            data_range: data_view.range,
            last_rva: address,
            rvas: Default::default(),
            rows: Default::default(),
            scroll_to_row: Default::default(),
        }
    }
}

impl AppView for AssemblyView {
    fn title(&self) -> String {
        format!("Assembly ({:016X})", self.rva).into()
    }

    fn ui(&mut self, context: &mut AppContext, ui: &mut Ui) {
        let row_height = ui.text_style_height(&TextStyle::Monospace);
        let mut table_builder = TableBuilder::new(ui)
            .min_scrolled_height(0.0)
            .max_scroll_height(f32::INFINITY)
            .column(Column::auto())
            .columns(Column::auto().resizable(true), 2)
            .column(Column::remainder());
        table_builder = if let Some(row) = self.scroll_to_row {
            self.scroll_to_row = None;
            table_builder.scroll_to_row(row, Some(Align::TOP))
        } else {
            table_builder
        };
        table_builder.body(|mut body| {
            let data = &context.project.data()[self.data_range.clone()];
            let style = body.ui_mut().style().clone();
            body.rows(row_height, self.rows.len() + 100, |index, mut row| {
                let instruction = if let Some(instruction) = self.rows.get(index) {
                    instruction
                } else {
                    // decode instruction
                    let rva = self.last_rva;
                    let position = (rva - self.rva) as usize;
                    let mut decoder = Decoder::with_ip(64, data, rva, DecoderOptions::NONE);
                    decoder.set_position(position).unwrap();
                    let instruction = decoder.decode();
                    self.last_rva += (decoder.position() - position) as u64;

                    // format instruction
                    let mut row_ = Row::new(
                        instruction.ip(),
                        data[position..position + instruction.len()]
                            .iter()
                            .map(|&elem| format!("{:02X}", elem))
                            .collect::<Vec<_>>()
                            .join(" "),
                    );
                    NasmFormatter::new().format(&instruction, &mut row_);
                    row_.post_format(&style);
                    self.rvas.insert(row_.rva);
                    self.rows.push(row_);

                    // validate addresses
                    for instruction in &mut self.rows {
                        for (text, rva) in &mut instruction.instruction {
                            let Some(rva) = rva else {
                                continue;
                            };
                            if (self.rva..self.last_rva).contains(rva) {
                                if !self.rvas.contains(rva) {
                                    text.sections.first_mut().unwrap().format.background =
                                        Color32::DARK_RED;
                                }
                            }
                        }
                    }
                    &self.rows[index]
                };
                let label = context.project.label_by_rva.get(&instruction.rva);

                // address column
                row.col(|ui| {
                    ui.add(
                        Label::new(RichText::from(format!("{:016X}", instruction.rva)).monospace())
                            .wrap(false)
                            .sense(Sense::click()),
                    )
                    .context_menu(|ui| {
                        ui.menu_button("Copy", |ui| {
                            if ui.button("VA").clicked() {
                                ui.close_menu();
                                ui.output_mut(|output| {
                                    output.copied_text =
                                        format!("{:016X}", context.project.base() + instruction.rva)
                                });
                            }
                            if ui.button("RVA").clicked() {
                                ui.close_menu();
                                ui.output_mut(|output| {
                                    output.copied_text = format!("{:016X}", instruction.rva)
                                });
                            }
                            if ui.button("Instruction").clicked() {
                                ui.close_menu();
                                ui.output_mut(|output| {
                                    output.copied_text = instruction
                                        .instruction
                                        .iter()
                                        .map(|(text, _)| text.text.as_str())
                                        .collect()
                                });
                            }
                        });
                    });
                });

                // bytes column
                row.col(|ui| {
                    ui.label(instruction.bytes.clone());
                });

                // instruction column
                row.col(|ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::ZERO;
                        for (text, rva) in &instruction.instruction {
                            if let Some(rva) = *rva {
                                if ui
                                    .add(Label::new(text.clone()).sense(Sense::click()))
                                    .clicked()
                                {
                                    if let Some(row) = self.rows.iter().enumerate().find_map(
                                        |(row, instruction)| {
                                            if instruction.rva == rva {
                                                Some(row)
                                            } else {
                                                None
                                            }
                                        },
                                    ) {
                                        self.scroll_to_row = Some(row);
                                    } else {
                                        context.open_address_view(rva);
                                    }
                                }
                            } else {
                                ui.label(text.clone());
                            }
                        }
                    });
                });

                // comment column
                row.col(|ui| {
                    if let Some(label) = label {
                        let mut layout_job = LayoutJob::simple_singleline(
                            format!("{}", label),
                            TextStyle::Monospace.resolve(&style),
                            style.visuals.text_color(),
                        );
                        layout_job.wrap = TextWrapping {
                            max_rows: 1,
                            ..Default::default()
                        };
                        ui.label(layout_job);
                    }
                });
            });
        });
    }
}

struct Row {
    rva: u64,
    bytes: LayoutJob,
    instruction: Vec<(LayoutJob, Option<u64>)>,
}

impl Row {
    fn new(address: u64, raw: String) -> Self {
        Self {
            rva: address,
            bytes: LayoutJob::simple_singleline(
                raw.clone(),
                FontId::default(),
                Color32::TEMPORARY_COLOR,
            ),
            instruction: Default::default(),
        }
    }

    fn post_format(&mut self, style: &Style) {
        // bytes column
        let text_format = &mut self.bytes.sections.first_mut().unwrap().format;
        text_format.font_id = TextStyle::Monospace.resolve(style);
        if text_format.color == Color32::TEMPORARY_COLOR {
            text_format.color = style.visuals.text_color()
        }
        self.bytes.wrap = TextWrapping {
            max_rows: 1,
            ..Default::default()
        };

        // instruction column
        for (text, _) in &mut self.instruction {
            let text_format = &mut text.sections.first_mut().unwrap().format;
            text_format.font_id = TextStyle::Monospace.resolve(style);
            if text_format.color == Color32::TEMPORARY_COLOR {
                text_format.color = style.visuals.text_color()
            }
            text.wrap = TextWrapping {
                max_rows: 1,
                ..Default::default()
            };
        }
    }
}

impl iced_x86::FormatterOutput for Row {
    fn write(&mut self, text: &str, kind: FormatterTextKind) {
        self.instruction.push(match kind {
            FormatterTextKind::LabelAddress | FormatterTextKind::FunctionAddress => (
                LayoutJob::simple_singleline(
                    text.to_string(),
                    FontId::default(),
                    Color32::TEMPORARY_COLOR,
                ),
                Some(u64::from_str_radix(&text[..text.len() - 1], 16).unwrap()),
            ),
            _ => (
                LayoutJob::simple_singleline(
                    text.to_string(),
                    FontId::default(),
                    match kind {
                        FormatterTextKind::Mnemonic => Color32::LIGHT_RED,
                        FormatterTextKind::Number => Color32::LIGHT_GREEN,
                        FormatterTextKind::Register => Color32::LIGHT_BLUE,
                        _ => Color32::WHITE,
                    },
                ),
                None,
            ),
        });
    }
}
