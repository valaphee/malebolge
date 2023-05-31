use std::{collections::HashSet, fmt::Display, path::PathBuf};

use boa_engine::{prelude::*, property::Attribute};
use clap::{Args, ValueEnum};
use colored::Colorize;
use iced_x86::{
    Decoder, DecoderOptions, Formatter, FormatterTextKind, GasFormatter, Instruction,
    IntelFormatter, MasmFormatter, NasmFormatter,
};
use object::{Architecture, Object, ObjectSection, ReadRef, SectionKind};

use crate::cli::print_table;

#[derive(Args)]
pub struct DumpArgs {
    #[arg(long, required = true, value_delimiter = ',')]
    pub columns: Vec<DumpArgsColumn>,

    #[arg(long)]
    pub offset: String,
    #[arg(long)]
    pub limit: Option<usize>,

    #[arg(long, default_value_t = Default::default())]
    pub format: DumpArgsFormat,
}

#[derive(ValueEnum, Clone, Hash, Eq, PartialEq)]
pub enum DumpArgsColumn {
    Idx,
    Off,
    Va,
    Rva,
    Raw,
    Asm,
}

#[derive(Default, ValueEnum, Clone)]
pub enum DumpArgsFormat {
    Plain,
    #[default]
    Color,
    Boxed,
}

impl Display for DumpArgsFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DumpArgsFormat::Plain => f.write_str("plain"),
            DumpArgsFormat::Color => f.write_str("color"),
            DumpArgsFormat::Boxed => f.write_str("boxed"),
        }
    }
}

pub(super) fn run(path: PathBuf, args: DumpArgs) {
    let mut address_context = Context::default();

    // parse file
    let data = std::fs::read(path).unwrap();
    let object = object::read::File::parse(data.as_slice()).unwrap();
    let base = object.relative_address_base();
    // ...and fill in address context
    address_context.register_global_property("base", base, Attribute::default());
    address_context.register_global_property("entry_point", object.entry(), Attribute::default());
    let js_section = JsObject::empty();
    for section in object.sections() {
        js_section
            .set(
                section.name().unwrap().trim_start_matches('.'),
                section.address(),
                false,
                &mut address_context,
            )
            .unwrap();
    }
    address_context.register_global_property("section", js_section, Attribute::default());

    // find section
    let va = address_context
        .eval(args.offset.clone())
        .unwrap()
        .as_number()
        .unwrap() as u64;
    let section = object
        .sections()
        .find(|section| {
            let section_address_begin = section.address();
            let section_address_end = section_address_begin + section.size();
            (section_address_begin..section_address_end).contains(&va)
        })
        .unwrap();

    run_internal(
        &section.data().unwrap()[(va - section.address()) as usize..],
        base,
        va,
        object.architecture(),
        args,
    )
}

pub(super) fn run_internal(
    data: &[u8],
    base: u64,
    va: u64,
    architecture: Architecture,
    args: DumpArgs,
) {
    let columns = args.columns.iter().collect::<HashSet<_>>();
    let rows = if columns.contains(&DumpArgsColumn::Asm) {
        // create asm decoder and formatter
        let mut asm_decoder = Decoder::with_ip(
            match architecture {
                Architecture::I386 => 32,
                Architecture::X86_64 => 64,
                _ => todo!(),
            },
            data,
            va,
            DecoderOptions::NONE,
        );
        let mut asm_formatter = AsmFormatter::Nasm(NasmFormatter::default());
        let mut instruction = Instruction::default();

        // generate table
        let mut index = 0;
        let limit = args.limit.unwrap_or(usize::MAX).min(data.len());
        let mut rows = Vec::with_capacity(limit);
        while index < limit && asm_decoder.can_decode() {
            asm_decoder.decode_out(&mut instruction);
            rows.push(
                args.columns
                    .iter()
                    .map(|column| match column {
                        DumpArgsColumn::Idx => format!("{}", index),
                        DumpArgsColumn::Off => format!("{:X}", instruction.ip() - va),
                        DumpArgsColumn::Va => format!("{:016X}", instruction.ip()),
                        DumpArgsColumn::Rva => format!("{:016X}", instruction.ip() - base),
                        DumpArgsColumn::Raw => data[(instruction.ip() - va) as usize..]
                            [..instruction.len()]
                            .iter()
                            .map(|element| format!("{:02X}", element))
                            .collect::<Vec<_>>()
                            .join(" "),
                        DumpArgsColumn::Asm => {
                            if matches!(args.format, DumpArgsFormat::Color) {
                                let mut output = FormatterOutput::default();
                                asm_formatter.format(&instruction, &mut output);
                                output.0
                            } else {
                                let mut output = String::default();
                                asm_formatter.format(&instruction, &mut output);
                                output
                            }
                        }
                    })
                    .collect::<Vec<_>>(),
            );
            index += 1;
        }
        rows
    } else {
        let aligned_va = va as usize / 16;
        let aligned_va_offset = va as usize % 16;

        // generate table
        let mut index = 0;
        let limit = args
            .limit
            .unwrap_or(usize::MAX)
            .min((data.len() + aligned_va_offset).div_ceil(16));
        let mut rows = Vec::with_capacity(limit);
        while index < limit {
            rows.push(
                args.columns
                    .iter()
                    .map(|column| match column {
                        DumpArgsColumn::Idx => format!("{:X}", index),
                        DumpArgsColumn::Off => format!("{:X}", index * 16),
                        DumpArgsColumn::Va => format!("{:X}", (index + aligned_va) * 16),
                        DumpArgsColumn::Rva => {
                            format!("{:X}", (index + aligned_va) * 16 - base as usize)
                        }
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
                                text =
                                    format!("{}{}", "   ".repeat(aligned_va_offset as usize), text);
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
    if matches!(
        args.format,
        DumpArgsFormat::Plain | DumpArgsFormat::Color | DumpArgsFormat::Boxed
    ) {
        print_table(
            args.columns
                .iter()
                .map(|column| {
                    match column {
                        DumpArgsColumn::Idx => "IDX",
                        DumpArgsColumn::Off => "OFF",
                        DumpArgsColumn::Va => "VA",
                        DumpArgsColumn::Rva => "RVA",
                        DumpArgsColumn::Raw => "RAW",
                        DumpArgsColumn::Asm => "ASM",
                    }
                    .to_string()
                })
                .collect(),
            rows,
            matches!(args.format, DumpArgsFormat::Boxed),
        );
    }
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
