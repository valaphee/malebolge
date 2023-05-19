use std::collections::HashSet;

use eframe::{
    egui::{
        text::LayoutJob, Align, Color32, FontId, Label, RichText, Sense, Style, TextStyle, Ui, Vec2,
    },
    epaint::text::TextWrapping,
};
use egui_extras::{Column, TableBuilder};
use iced_x86::{Decoder, DecoderOptions, Formatter, FormatterTextKind, NasmFormatter};

use crate::{project::Project, tab::Tab, util::warden, LabelWindow};

pub struct AssemblyTab {
    bitness: u32,
    address: u64,
    data_offset: usize,
    data_length: usize,
    // runtime
    last_address: u64,
    addresses: HashSet<u64>,
    instructions: Vec<Instruction>,
    go_to_row: Option<usize>,
}

impl AssemblyTab {
    pub fn new(bitness: u32, address: u64, data_offset: usize, data_length: usize) -> Self {
        Self {
            bitness,
            address,
            data_offset,
            data_length,
            last_address: address,
            addresses: Default::default(),
            instructions: Default::default(),
            go_to_row: None,
        }
    }
}

impl Tab for AssemblyTab {
    fn title(&self) -> String {
        format!("Assembly ({:016X})", self.address).into()
    }

    fn ui(&mut self, project: &mut Project, ui: &mut Ui) {
        // render table
        let row_height = ui.text_style_height(&TextStyle::Monospace);
        let style = ui.style().clone();
        let mut table_builder = TableBuilder::new(ui)
            .min_scrolled_height(0.0)
            .max_scroll_height(f32::INFINITY)
            .column(Column::auto())
            .columns(Column::auto().resizable(true), 2)
            .column(Column::remainder());
        table_builder = if let Some(row) = self.go_to_row {
            self.go_to_row = None;
            table_builder.scroll_to_row(row, Some(Align::TOP))
        } else {
            table_builder
        };
        table_builder.body(|body| {
            // render rows
            body.rows(
                row_height,
                self.instructions.len() + 100,
                |index, mut row| {
                    // cache decoded instructions, rows will always be loaded in order, therefore
                    // its save to use a Vec
                    let instruction = if let Some(instruction) = self.instructions.get(index) {
                        instruction
                    } else {
                        // decode instruction
                        let data =
                            &project.data[self.data_offset..self.data_offset + self.data_length];
                        let address = self.last_address;
                        let position = (address - self.address) as usize;
                        let mut decoder =
                            Decoder::with_ip(self.bitness, data, address, DecoderOptions::NONE);
                        decoder.set_position(position).unwrap();
                        let raw_instruction = warden::CfoPatcher::new(&mut decoder).next().unwrap();
                        self.last_address += (decoder.position() - position) as u64;
                        // format instruction
                        let mut instruction = Instruction::new(
                            raw_instruction.ip(),
                            data[position..position + raw_instruction.len()]
                                .iter()
                                .map(|&elem| format!("{:02X}", elem))
                                .collect::<Vec<_>>()
                                .join(" "),
                        );
                        NasmFormatter::new().format(&raw_instruction, &mut instruction);
                        instruction.post_format(&style);
                        self.addresses.insert(instruction.address);
                        self.instructions.push(instruction);
                        // validate addresses
                        for instruction in &mut self.instructions {
                            for (text, address) in &mut instruction.instruction {
                                let Some(address) = address else {
                                    continue;
                                };
                                if (self.address..self.last_address).contains(address) {
                                    if !self.addresses.contains(address) {
                                        text.sections.first_mut().unwrap().format.background =
                                            Color32::DARK_RED;
                                    }
                                }
                            }
                        }
                        &self.instructions[index]
                    };
                    // get label
                    let label = project.labels.get(&instruction.address);
                    // render address column
                    row.col(|ui| {
                        ui.add(
                            Label::new(
                                RichText::from(format!("{:016X}", instruction.address)).monospace(),
                            )
                            .wrap(false)
                            .sense(Sense::click()),
                        )
                        .context_menu(|ui| {
                            ui.menu_button("Copy", |ui| {
                                if ui.button("VA").clicked() {
                                    ui.output_mut(|output| {
                                        output.copied_text = format!("{:016X}", instruction.address)
                                    });
                                    ui.close_menu();
                                }
                                if ui.button("Instruction").clicked() {
                                    ui.output_mut(|output| {
                                        output.copied_text = instruction
                                            .instruction
                                            .iter()
                                            .map(|(text, _)| text.text.as_str())
                                            .collect()
                                    });
                                    ui.close_menu();
                                }
                            });
                            if ui.button("Label").clicked() && project.label_window.is_none() {
                                project.label_window = Some(LabelWindow::new(
                                    label.map_or("".to_string(), |label| label.name.clone()),
                                    instruction.address,
                                ));
                                ui.close_menu();
                            }
                        });
                    });
                    // render raw column
                    row.col(|ui| {
                        ui.label(instruction.raw.clone());
                    });
                    // render instruction column
                    row.col(|ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::ZERO;
                            for (text, address_with_align_bit) in &instruction.instruction {
                                if let Some(address) = *address_with_align_bit {
                                    if ui
                                        .add(Label::new(text.clone()).sense(Sense::click()))
                                        .clicked()
                                    {
                                        if let Some(row) =
                                            self.instructions.iter().enumerate().find_map(
                                                |(row, instruction)| {
                                                    if instruction.address == address {
                                                        Some(row)
                                                    } else {
                                                        None
                                                    }
                                                },
                                            )
                                        {
                                            self.go_to_row = Some(row);
                                        } else {
                                            project.go_to_address = Some(address);
                                        }
                                    }
                                } else {
                                    ui.label(text.clone());
                                }
                            }
                        });
                    });
                    // render comment column
                    row.col(|ui| {
                        if let Some(label) = label {
                            let mut layout_job = LayoutJob::simple_singleline(
                                label.name.clone(),
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
                },
            );
        });
    }
}

struct Instruction {
    address: u64,
    raw: LayoutJob,
    instruction: Vec<(LayoutJob, Option<u64>)>,
}

impl Instruction {
    fn new(address: u64, raw: String) -> Self {
        Self {
            address,
            raw: LayoutJob::simple_singleline(
                raw.clone(),
                FontId::default(),
                Color32::TEMPORARY_COLOR,
            ),
            instruction: Default::default(),
        }
    }

    fn post_format(&mut self, style: &Style) {
        let text_format = &mut self.raw.sections.first_mut().unwrap().format;
        text_format.font_id = TextStyle::Monospace.resolve(style);
        if text_format.color == Color32::TEMPORARY_COLOR {
            text_format.color = style.visuals.text_color()
        }
        self.raw.wrap = TextWrapping {
            max_rows: 1,
            ..Default::default()
        };
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

impl iced_x86::FormatterOutput for Instruction {
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
