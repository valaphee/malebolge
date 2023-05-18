use std::collections::HashSet;

use eframe::egui::{Align, Color32, Label, Layout, RichText, Sense, TextStyle, Ui, Vec2};
use egui_extras::{Column, TableBuilder};
use iced_x86::{Decoder, DecoderOptions, Formatter, FormatterTextKind, NasmFormatter};

use crate::{warden, AppView, Global};

pub struct AssemblyView {
    bitness: u32,
    address: u64,
    data_offset: usize,
    data_length: usize,
    // runtime
    last_address: u64,
    addresses: HashSet<u64>,
    instructions: Vec<Instruction>,
    // event
    go_to_row: Option<usize>,
}

impl AssemblyView {
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

impl AppView for AssemblyView {
    fn title(&self) -> String {
        format!("Assembly ({:016X})", self.address).into()
    }

    fn ui(&mut self, state: &mut Global, ui: &mut Ui) {
        // render table
        let row_height = ui.text_style_height(&TextStyle::Monospace);
        let mut table_builder = TableBuilder::new(ui)
            .min_scrolled_height(0.0)
            .max_scroll_height(f32::INFINITY)
            .column(Column::auto())
            .column(Column::auto().resizable(true))
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
                        // decode and format instruction
                        let data =
                            &state.data[self.data_offset..self.data_offset + self.data_length];
                        let address = self.last_address;
                        let position = (address - self.address) as usize;
                        let mut decoder =
                            Decoder::with_ip(self.bitness, data, address, DecoderOptions::NONE);
                        decoder.set_position(position).unwrap();
                        let raw_instruction = warden::CfoPatcher::new(&mut decoder).next().unwrap();
                        let mut instruction = Instruction::new(
                            raw_instruction.ip(),
                            data[position..position + raw_instruction.len()]
                                .iter()
                                .map(|&elem| format!("{:02X}", elem))
                                .collect::<Vec<_>>()
                                .join(" "),
                        );
                        NasmFormatter::new().format(&raw_instruction, &mut instruction);
                        self.last_address += (decoder.position() - position) as u64;
                        self.addresses.insert(instruction.address);
                        self.instructions.push(instruction);
                        // validate addresses
                        for instruction in &mut self.instructions {
                            for (text, address) in &mut instruction.text {
                                let Some(address) = address else {
                                    continue;
                                };
                                if (self.address..self.last_address).contains(address) {
                                    if !self.addresses.contains(address) {
                                        *text = text.clone().background_color(Color32::DARK_RED);
                                    }
                                }
                            }
                        }
                        &self.instructions[index]
                    };
                    // render cols
                    row.col(|ui| {
                        ui.add(
                            Label::new(
                                RichText::from(format!("{:016X}", instruction.address)).monospace(),
                            )
                            .wrap(false),
                        );
                    });
                    row.col(|ui| {
                        ui.add(
                            Label::new(RichText::from(&instruction.data).monospace()).wrap(false),
                        );
                    });
                    row.col(|ui| {
                        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                            ui.spacing_mut().item_spacing = Vec2::ZERO;
                            for (text, address_with_align_bit) in &instruction.text {
                                if let Some(address) = *address_with_align_bit {
                                    if ui
                                        .add(
                                            Label::new(text.clone())
                                                .wrap(false)
                                                .sense(Sense::click()),
                                        )
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
                                            state.go_to_address = Some(address);
                                        }
                                    }
                                } else {
                                    ui.add(Label::new(text.clone()).wrap(false));
                                }
                            }
                        });
                    });
                },
            );
        });
    }
}

struct Instruction {
    address: u64,
    data: String,
    text: Vec<(RichText, Option<u64>)>,
}

impl Instruction {
    fn new(address: u64, data: String) -> Self {
        Self {
            address,
            data,
            text: Default::default(),
        }
    }
}

impl iced_x86::FormatterOutput for Instruction {
    fn write(&mut self, text: &str, kind: FormatterTextKind) {
        self.text.push(match kind {
            FormatterTextKind::LabelAddress | FormatterTextKind::FunctionAddress => (
                RichText::from(text).monospace(),
                Some(u64::from_str_radix(&text[..text.len() - 1], 16).unwrap()),
            ),
            _ => (
                RichText::from(text)
                    .color(match kind {
                        FormatterTextKind::Mnemonic => Color32::LIGHT_RED,
                        FormatterTextKind::Number => Color32::LIGHT_GREEN,
                        FormatterTextKind::Register => Color32::LIGHT_BLUE,
                        _ => Color32::WHITE,
                    })
                    .monospace(),
                None,
            ),
        });
    }
}
