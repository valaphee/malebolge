use std::{collections::HashSet, path::PathBuf};

use clap::{Args, ValueEnum};
use colored::Colorize;
use iced_x86::{
    Decoder, DecoderOptions, Formatter, FormatterTextKind, GasFormatter, Instruction,
    IntelFormatter, MasmFormatter, NasmFormatter,
};
use object::Architecture;

use crate::{cli::output::Output, win::process::Process};

#[derive(Args)]
pub struct DumpArgs {
    count: usize,

    #[arg(long, required = true, value_delimiter = ',')]
    format: Vec<DumpArgsColumn>,
}

#[derive(ValueEnum, Clone, Hash, Eq, PartialEq)]
pub enum DumpArgsColumn {
    Idx,
    Off,
    Va,
    Raw,
    Asm,
}

pub fn run(process: &Process, address: usize, args: DumpArgs) {
    let data = process.read(address, args.count);
    let data = data.as_slice();
    let format = args.format.iter().collect::<HashSet<_>>();
    let rows = if format.contains(&DumpArgsColumn::Asm) {
        // create asm decoder and formatter
        let mut asm_decoder = Decoder::with_ip(64, data, address as u64, DecoderOptions::NONE);
        let mut asm_formatter = AsmFormatter::Nasm(NasmFormatter::default());
        let mut instruction = Instruction::default();

        // generate table
        let mut index = 0;
        let mut rows = vec![];
        while asm_decoder.can_decode() {
            asm_decoder.decode_out(&mut instruction);
            rows.push(
                args.format
                    .iter()
                    .map(|column| match column {
                        DumpArgsColumn::Idx => format!("{}", index),
                        DumpArgsColumn::Off => format!("{:X}", instruction.ip() - address as u64),
                        DumpArgsColumn::Va => format!("{:016X}", instruction.ip()),
                        DumpArgsColumn::Raw => data[(instruction.ip() - address as u64) as usize..]
                            [..instruction.len()]
                            .iter()
                            .map(|element| format!("{:02X}", element))
                            .collect::<Vec<_>>()
                            .join(" "),
                        DumpArgsColumn::Asm => {
                            let mut output = FormatterOutput::default();
                            asm_formatter.format(&instruction, &mut output);
                            output.0
                        }
                    })
                    .collect::<Vec<_>>(),
            );
            index += 1;
        }
        rows
    } else {
        let aligned_va = address / 16;
        let aligned_va_offset = address % 16;

        // generate table
        let mut index = 0;
        let limit = args
            .count
            .min((data.len() + aligned_va_offset).div_ceil(16));
        let mut rows = Vec::with_capacity(limit);
        while index < limit {
            rows.push(
                args.format
                    .iter()
                    .map(|column| match column {
                        DumpArgsColumn::Idx => format!("{:X}", index),
                        DumpArgsColumn::Off => format!("{:X}", index * 16),
                        DumpArgsColumn::Va => format!("{:X}", (index + aligned_va) * 16),
                        DumpArgsColumn::Raw => {
                            let mut text = data[if index == 0 {
                                0
                            } else {
                                index * 16 - aligned_va_offset
                            }
                                ..(index * 16 + 16 - aligned_va_offset)]
                                .iter()
                                .map(|&elem| format!("{:02X}", elem))
                                .collect::<Vec<_>>()
                                .join(" ");
                            if index == 0 {
                                text = format!("{}{}", "   ".repeat(aligned_va_offset), text);
                            }
                            text
                        }
                        _ => todo!(),
                    })
                    .collect::<Vec<_>>(),
            );
            index += 1;
        }
        rows
    };

    // print table
    Output::Pretty.print_table(
        args.format
            .iter()
            .map(|column| {
                match column {
                    DumpArgsColumn::Idx => "IDX",
                    DumpArgsColumn::Off => "OFF",
                    DumpArgsColumn::Va => "VA",
                    DumpArgsColumn::Raw => "RAW",
                    DumpArgsColumn::Asm => "ASM",
                }
                .to_string()
            })
            .collect(),
        rows,
    );
}

enum AsmFormatter {
    Gas(GasFormatter),
    Intel(IntelFormatter),
    Masm(MasmFormatter),
    Nasm(NasmFormatter),
}

impl AsmFormatter {
    fn format(&mut self, instruction: &Instruction, output: &mut impl iced_x86::FormatterOutput) {
        match self {
            AsmFormatter::Gas(formatter) => formatter.format(instruction, output),
            AsmFormatter::Intel(formatter) => formatter.format(instruction, output),
            AsmFormatter::Masm(formatter) => formatter.format(instruction, output),
            AsmFormatter::Nasm(formatter) => formatter.format(instruction, output),
        }
    }
}

#[derive(Default)]
struct FormatterOutput(String);

impl iced_x86::FormatterOutput for FormatterOutput {
    fn write(&mut self, text: &str, kind: FormatterTextKind) {
        self.0.push_str(&match kind {
            FormatterTextKind::Mnemonic => text.bright_red().to_string(),
            FormatterTextKind::Keyword => text.red().to_string(),
            FormatterTextKind::Number => text.bright_blue().to_string(),
            FormatterTextKind::Register => text.bright_green().to_string(),
            _ => text.to_string(),
        });
    }
}
