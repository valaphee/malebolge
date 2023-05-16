use eframe::egui::{Align, Color32, Label, Layout, RichText, Sense, TextStyle, Ui, Vec2};
use egui_extras::{Column, TableBuilder};
use iced_x86::{Decoder, DecoderOptions, Formatter, FormatterTextKind, NasmFormatter};
use once_cell::sync::Lazy;

use crate::{AppState, AppView};

pub struct AssemblyView {
    bitness: u32,
    address: u64,
    data_offset: usize,
    data_length: usize,
    // cache
    last_address: u64,
    instructions: Vec<Instruction>,
}

impl AssemblyView {
    pub fn new(bitness: u32, address: u64, data_offset: usize, data_length: usize) -> Self {
        Self {
            bitness,
            address,
            data_offset,
            data_length,
            last_address: address,
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
            .column(Column::auto().resizable(true))
            .column(Column::remainder())
            .body(|body| {
                let address = self.last_address;
                let position = (address - self.address) as usize;
                let mut decoder_and_formatter =
                    Lazy::<(iced_x86::Instruction, Decoder, NasmFormatter), _>::new(|| {
                        let raw_instruction = iced_x86::Instruction::default();
                        let mut decoder = Decoder::with_ip(
                            self.bitness,
                            &state.data[self.data_offset..self.data_offset + self.data_length],
                            address,
                            DecoderOptions::NONE,
                        );
                        decoder.set_position(position).unwrap();
                        (raw_instruction, decoder, NasmFormatter::new())
                    });
                body.rows(row_height, self.instructions.len() + 1, |index, mut row| {
                    let instruction = if let Some(instruction) = self.instructions.get(index) {
                        instruction
                    } else {
                        let (ref mut raw_instruction, ref mut decoder, ref mut formatter) =
                            *decoder_and_formatter;
                        decoder.decode_out(raw_instruction);
                        self.last_address += raw_instruction.len() as u64;
                        let mut instruction = Instruction::new(raw_instruction.ip());
                        formatter.format(raw_instruction, &mut instruction);
                        self.instructions.push(instruction);
                        &self.instructions[index]
                    };
                    row.col(|ui| {
                        ui.add(
                            Label::new(
                                RichText::from(format!("{:016X}", instruction.address)).monospace(),
                            )
                            .wrap(false),
                        );
                    });
                    row.col(|ui| {
                        ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
                            ui.spacing_mut().item_spacing = Vec2::ZERO;
                            for (text, address) in &instruction.text {
                                if let Some(address) = address {
                                    if ui
                                        .add(
                                            Label::new(text.clone())
                                                .wrap(false)
                                                .sense(Sense::click()),
                                        )
                                        .clicked()
                                    {
                                        state.go_to_address = Some(*address);
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

struct Instruction {
    address: u64,
    text: Vec<(RichText, Option<u64>)>,
}

impl Instruction {
    fn new(address: u64) -> Self {
        Self {
            address,
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
