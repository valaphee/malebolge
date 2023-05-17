use std::collections::HashSet;
use eframe::egui::{Align, Color32, Label, Layout, RichText, Sense, TextStyle, Ui, Vec2};
use egui_extras::{Column, TableBuilder};
use iced_x86::{Decoder, DecoderOptions, Formatter, FormatterTextKind, NasmFormatter};

use crate::{AppState, AppView};

pub struct AssemblyView {
    bitness: u32,
    address: u64,
    data_offset: usize,
    data_length: usize,
    // cache
    last_address: u64,
    addresses: HashSet<u64>,
    instructions: Vec<CachedInstruction>,
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
        }
    }
}

impl AppView for AssemblyView {
    fn title(&self) -> String {
        format!("Assembly ({:016X})", self.address).into()
    }

    fn ui(&mut self, state: &mut AppState, ui: &mut Ui) {
        let row_height = ui.text_style_height(&TextStyle::Monospace);
        TableBuilder::new(ui)
            .min_scrolled_height(0.0)
            .max_scroll_height(f32::INFINITY)
            .columns(Column::auto(), 2)
            .column(Column::remainder())
            .body(|body| {
                // render rows
                body.rows(row_height, self.instructions.len() + 1, |index, mut row| {
                    // cache decoded instructions, rows will always be loaded in order, therefore
                    // its save to use a Vec
                    let instruction = if let Some(instruction) = self.instructions.get(index) {
                        instruction
                    } else {
                        let data = &state.data[self.data_offset..self.data_offset + self.data_length];
                        let address = self.last_address;
                        let position = (address - self.address) as usize;
                        let mut decoder = Decoder::with_ip(self.bitness, data, address, DecoderOptions::NONE);
                        decoder.set_position(position).unwrap();
                        // decode and format instruction
                        let instruction = decoder.decode();
                        let mut cached_instruction = CachedInstruction::new(
                            instruction.ip(),
                            data[position..decoder.position()]
                                .iter()
                                .map(|&elem| format!("{:02X}", elem))
                                .collect::<Vec<_>>()
                                .join(" "),
                        );
                        NasmFormatter::new().format(&instruction, &mut cached_instruction);
                        self.last_address += instruction.len() as u64;
                        self.addresses.insert(cached_instruction.address);
                        self.instructions.push(cached_instruction);
                        // validate address
                        for cached_instruction in &mut self.instructions {
                            for (text, go_to_address) in &mut cached_instruction.text {
                                let Some(go_to_address) = go_to_address.to_owned() else {
                                    continue;
                                };
                                if (self.address..self.last_address).contains(&go_to_address) && !self.addresses.contains(&go_to_address) {
                                    *text = text.clone().background_color(Color32::DARK_RED);
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
                            for (text, go_to_address) in &instruction.text {
                                if let Some(go_to_address) = go_to_address {
                                    if ui
                                        .add(
                                            Label::new(text.clone())
                                                .wrap(false)
                                                .sense(Sense::click()),
                                        )
                                        .clicked()
                                    {
                                        state.go_to_address = Some(*go_to_address);
                                    }
                                } else {
                                    ui.add(Label::new(text.clone()).wrap(false));
                                }
                            }
                        });
                    });
                });
            });
    }
}

struct CachedInstruction {
    address: u64,
    data: String,
    text: Vec<(RichText, Option<u64>)>,
}

impl CachedInstruction {
    fn new(address: u64, data: String) -> Self {
        Self {
            address,
            data,
            text: Default::default(),
        }
    }
}

impl iced_x86::FormatterOutput for CachedInstruction {
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
