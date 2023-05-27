use std::{fmt::Display, path::PathBuf};

use boa_engine::{object::JsMap, prelude::*, property::Attribute};
use clap::{Args, ValueEnum};
use colored::Colorize;
use iced_x86::{
    Decoder, DecoderOptions, Formatter, FormatterTextKind, GasFormatter, Instruction,
    IntelFormatter, MasmFormatter, NasmFormatter,
};
use object::{Architecture, Object, ObjectSection, SectionKind};

use crate::cli::print_table;

/// View file
#[derive(Args)]
pub struct ViewArgs {
    /// File
    path: PathBuf,
    /// Columns
    #[arg(long, required = true, value_delimiter = ',')]
    columns: Vec<ViewArgsColumn>,

    /// Where
    #[arg(long)]
    at: String,
    /// Row limit
    #[arg(long)]
    limit: u64,

    /// Format
    #[arg(long, default_value_t = Default::default())]
    format: ViewArgsFormat,
}

#[derive(ValueEnum, Clone)]
pub enum ViewArgsColumn {
    Index,
    Offset,
    Raw,

    // assembly
    Va,
    Rva,
    Asm,
}

#[derive(Default, ValueEnum, Clone)]
pub enum ViewArgsFormat {
    Plain,
    #[default]
    Color,
    Boxed,
}

impl Display for ViewArgsFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ViewArgsFormat::Plain => f.write_str("plain"),
            ViewArgsFormat::Color => f.write_str("color"),
            ViewArgsFormat::Boxed => f.write_str("boxed"),
        }
    }
}

pub(super) fn run(args: ViewArgs) {
    let mut address_context = Context::default();

    // parse file
    let data = std::fs::read(args.path).unwrap();
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

    // evaluate "at" using position context
    println!("{:?}", address_context.eval(args.at.clone()).unwrap());
    let at = address_context.eval(args.at).unwrap().as_number().unwrap() as u64;

    // find section
    let section = object
        .sections()
        .find(|section| {
            let section_address_begin = section.address();
            let section_address_end = section_address_begin + section.size();
            (section_address_begin..section_address_end).contains(&at)
        })
        .unwrap();
    let section_data = &section.data().unwrap()[(at - section.address()) as usize..];

    let rows = if section.kind() == SectionKind::Text {
        // create asm decoder and formatter
        let mut asm_decoder = Decoder::with_ip(
            match object.architecture() {
                Architecture::I386 => 32,
                Architecture::X86_64 => 64,
                _ => todo!(),
            },
            section_data,
            at,
            DecoderOptions::NONE,
        );
        let mut asm_formatter = AsmFormatter::Nasm(NasmFormatter::default());
        let mut instruction = Instruction::default();

        // generate table
        (0..args.limit)
            .into_iter()
            .map(|index| {
                asm_decoder.decode_out(&mut instruction);
                args.columns
                    .iter()
                    .map(|column| match column {
                        ViewArgsColumn::Index => format!("{:X}", index),
                        ViewArgsColumn::Offset => format!("{:X}", instruction.ip() - at),
                        ViewArgsColumn::Raw => section_data[(instruction.ip() - at) as usize..]
                            [..instruction.len()]
                            .iter()
                            .map(|element| format!("{:02X}", element))
                            .collect::<Vec<_>>()
                            .join(" "),
                        ViewArgsColumn::Va => format!("{:016X}", instruction.ip()),
                        ViewArgsColumn::Rva => format!("{:016X}", instruction.ip() - base),
                        ViewArgsColumn::Asm => {
                            if matches!(args.format, ViewArgsFormat::Color) {
                                let mut output = FormatterOutput::default();
                                asm_formatter.format(&instruction, &mut output);
                                output.0
                            } else {
                                let mut output = String::default();
                                asm_formatter.format(&instruction, &mut output);
                                output
                            }
                        }
                        _ => todo!(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    } else {
        let aligned_at = at as usize / 16;
        let aligned_at_offset = at as usize % 16;

        // generate table
        (0..args.limit as usize)
            .into_iter()
            .map(|index| {
                args.columns
                    .iter()
                    .map(|column| match column {
                        ViewArgsColumn::Index => format!("{:X}", index),
                        ViewArgsColumn::Offset => format!("{:X}", index * 16),
                        ViewArgsColumn::Raw => {
                            let mut text = section_data[if index == 0 {
                                0
                            } else {
                                index * 16 - aligned_at_offset
                            }
                                ..(index * 16 + 16 - aligned_at_offset)]
                                .iter()
                                .map(|&elem| format!("{:02X}", elem))
                                .collect::<Vec<_>>()
                                .join(" ");
                            if index == 0 {
                                text =
                                    format!("{}{}", "   ".repeat(aligned_at_offset as usize), text);
                            }
                            text
                        }
                        ViewArgsColumn::Va => format!("{:X}", (index + aligned_at) * 16),
                        ViewArgsColumn::Rva => {
                            format!("{:X}", (index + aligned_at) * 16 - base as usize)
                        }
                        _ => todo!(),
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    };

    // print table
    if matches!(
        args.format,
        ViewArgsFormat::Plain | ViewArgsFormat::Color | ViewArgsFormat::Boxed
    ) {
        print_table(
            args.columns
                .iter()
                .map(|column| {
                    match column {
                        ViewArgsColumn::Index => "INDEX",
                        ViewArgsColumn::Offset => "OFFSET",
                        ViewArgsColumn::Raw => "RAW",
                        ViewArgsColumn::Asm => "ASSEMBLY",
                        ViewArgsColumn::Va => "VA",
                        ViewArgsColumn::Rva => "RVA",
                    }
                    .to_string()
                })
                .collect(),
            rows,
            matches!(args.format, ViewArgsFormat::Boxed),
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
