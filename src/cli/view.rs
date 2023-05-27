use std::path::PathBuf;
use boa_engine::Context;
use boa_engine::property::Attribute;
use clap::{Args, ValueEnum};
use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, NasmFormatter};
use object::{Architecture, Object, ObjectSection};

#[derive(Args)]
pub struct ViewArgs {
    path: PathBuf,
    address: String,
    #[arg(required = true, value_delimiter = ',')]
    columns: Vec<ViewArgsColumn>,
    count: u64,
}

#[derive(ValueEnum, Clone)]
pub enum ViewArgsColumn {
    Va,
    Rva,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    Nasm,
}

pub(super) fn run(args: ViewArgs) {
    let mut context = Context::default();

    let data = std::fs::read(args.path).unwrap();
    let object = object::read::File::parse(data.as_slice()).unwrap();
    let base = object.relative_address_base();
    context.register_global_property("base", base, Attribute::default());
    context.register_global_property("entry_point", object.entry(), Attribute::default());

    let address = context.eval(args.address).unwrap().as_number().unwrap() as u64;
    let section = object.sections().find(|section| {
        let section_address_begin = section.address();
        let section_address_end = section_address_begin + section.size();
        (section_address_begin..section_address_end).contains(&address)
    }).unwrap();
    let section_data = &section.data().unwrap()[(address - section.address()) as usize..];

    let mut asm_decoder = Decoder::with_ip(match object.architecture() {
        Architecture::X86_64 => 64,
        _ => todo!()
    }, section_data, address, DecoderOptions::NONE);
    let mut nasm_formatter = NasmFormatter::new();
    let mut instruction = Instruction::default();

    let mut column_widths = vec![0; args.columns.len()];
    let table = (0..args.count).into_iter().map(|_| {
        asm_decoder.decode_out(&mut instruction);
        args.columns.iter().enumerate().map(|(column_index, column)| {
            let cell = match column {
                ViewArgsColumn::Va => format!("{:016X}", instruction.ip()),
                ViewArgsColumn::Rva => format!("{:016X}", instruction.ip() - base),
                ViewArgsColumn::U8 => section_data[(instruction.ip() - address) as usize..][..instruction.len()].iter().map(|element| format!("{:02X}", element)).collect::<Vec<_>>().join(" "),
                ViewArgsColumn::Nasm => {
                    let mut output = String::new();
                    nasm_formatter.format(&instruction, &mut output);
                    output
                }
                _ => todo!()
            };
            if column_widths[column_index] < cell.len() {
                column_widths[column_index] = cell.len();
            }
            cell
        }).collect::<Vec<_>>()
    }).collect::<Vec<_>>();

    for row in table {
        println!("{}", row.iter().enumerate().map(|(column, cell)| {
            let column_width = column_widths[column];
            format!("{:<1$}", cell, column_width)
        }).collect::<Vec<_>>().join(" "));
    }
}
