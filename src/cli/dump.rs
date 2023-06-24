use std::path::PathBuf;

use clap::{Args, ValueEnum};
use colored::Colorize;
use iced_x86::{
    Formatter, FormatterTextKind, GasFormatter, Instruction, IntelFormatter, MasmFormatter,
    NasmFormatter,
};

#[derive(Args)]
pub struct DumpArgs {
    count: usize,

    #[arg(long, value_delimiter = ',')]
    show: Vec<DumpArgsColumn>,
}

#[derive(ValueEnum, Clone)]
pub enum DumpArgsColumn {
    Idx,
    Off,
    Raw,
    Va,
    Rva,
    Asm,
}

pub fn run(args: DumpArgs) {}

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
