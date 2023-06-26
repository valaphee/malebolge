use std::{io::Write, path::PathBuf};
use std::ops::Index;

use clap::Parser;
use windows::core::PCSTR;
use windows::s;
use windows::Win32::Foundation::{DUPLICATE_HANDLE_OPTIONS, DUPLICATE_SAME_ACCESS, DuplicateHandle, FALSE, HANDLE, INVALID_HANDLE_VALUE};
use windows::Win32::System::Memory::{CreateFileMappingA, FILE_MAP_READ, FILE_MAP_WRITE, MapViewOfFile, PAGE_READWRITE};
use windows::Win32::System::Threading::{CreateEventA, GetCurrentProcess, INFINITE, SetEvent, WaitForSingleObject};
use mbg_hook_shared::{BreakpointEntry, BreakpointList, BreakpointListView};

use crate::{cli::address::Address, win::process::Process};
use crate::win::breakpoint::{Breakpoint, BreakpointType};

mod address;

#[derive(Parser)]
pub struct Args {
    path: PathBuf,
}

#[derive(Parser)]
pub enum Command {
    #[command(alias = "q")]
    Quit,
    #[command(alias = "g")]
    Go { address: Address },
    #[command(alias = "b")]
    Break { address: Address },
    #[command(alias = "c")]
    Continue { count: usize },
    #[command(alias = "n")]
    Next { count: usize },
    #[command(alias = "s")]
    Step { count: usize },
    #[command(alias = "lm")]
    ListModules,
    #[command(alias = "ls")]
    ListSymbols,
    #[command(alias = "lt")]
    ListThreads,
}

pub fn run(args: Args) {
    let process = Process::new(args.path);
    let mut current_address = 0;

    let mut breakpoint_list_view = BreakpointListView::new(process.process);
    let breakpoint_list: &'static mut BreakpointList = unsafe { std::mem::transmute(breakpoint_list_view.data()) };
    let breakpoint_list_0: &'static mut BreakpointList = unsafe { std::mem::transmute(breakpoint_list_view.data()) };
    std::mem::forget(breakpoint_list_view);

    std::thread::spawn(|| {
        loop {
            unsafe {
                WaitForSingleObject(breakpoint_list_0.chld_event, INFINITE).ok().unwrap();
            }
            for entry in &mut breakpoint_list_0.entries {
                if entry.address == 0 || !entry.trigger {
                    continue;
                }
                println!("triggered {:016X}", entry.address);
                entry.trigger = false;
            }
            unsafe {
                SetEvent(breakpoint_list_0.prnt_event).ok().unwrap();
            }
        }
    });

    let tls_addr = process.modules().first().unwrap().symbol("tls_callback_0").unwrap();
    let breakpoint = Breakpoint::new(process.process, tls_addr as u64, BreakpointType::Int3).unwrap();
    breakpoint.enable();

    let free_entry = breakpoint_list.entries.iter().position(|entry| entry.address == 0).unwrap();
    breakpoint_list.entries[free_entry] = BreakpointEntry {
        address: tls_addr,
        trigger: false,
    };

    process.load_library("target\\debug\\mbg_hook.dll").unwrap();
    //process.resume();

    let mut input = String::new();
    loop {
        print!("{:016X}> ", current_address);
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut input).unwrap();
        match Command::try_parse_from(std::iter::once("").chain(input.trim().split(' '))) {
            Ok(value) => match value {
                Command::Quit => {
                    break;
                }
                Command::Go { address } => {
                    current_address = address.to_raw(&process).unwrap();
                }
                Command::Break { address: _ } => {}
                Command::Continue { count: _ } => {}
                Command::Next { count: _ } => {}
                Command::Step { count: _ } => {}
                Command::ListModules => {
                    for module in process.modules() {
                        println!(
                            "{}: {:016X} {:016X}",
                            module.name(),
                            module.base(),
                            module.size()
                        )
                    }
                }
                Command::ListSymbols => {
                    for module in process.modules() {
                        println!("{}:", module.name());
                        for (symbol_name, symbol) in module.symbols() {
                            println!("\t{}: {:016X}", symbol_name, symbol);
                        }
                    }
                }
                Command::ListThreads => {}
            },
            Err(error) => {
                let _ = error.print();
            }
        }
        input.clear();
        println!();
        std::io::stdout().flush().unwrap();
    }
}
