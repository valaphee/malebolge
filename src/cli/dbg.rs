use std::path::PathBuf;

use clap::{Args, Parser};
use windows::{
    core::{HSTRING, PCWSTR, PWSTR},
    Win32::{
        Foundation::DBG_CONTINUE,
        System::{
            Diagnostics::Debug::{
                ContinueDebugEvent, WaitForDebugEvent, CREATE_PROCESS_DEBUG_EVENT, DEBUG_EVENT,
            },
            Threading::{
                CreateProcessW, DEBUG_ONLY_THIS_PROCESS, DEBUG_PROCESS, INFINITE,
                PROCESS_INFORMATION, STARTUPINFOW,
            },
        },
    },
};

use crate::{
    cli::{dump, dump::DumpArgs, info},
    dbg::peb,
};

#[derive(Args)]
pub struct DbgArgs {
    pid: Option<u32>,
}

#[derive(Parser)]
pub enum DbgCommand {
    Exit,
    Info,
    Dump(DumpArgs),
}

#[allow(mutable_transmutes)]
pub(super) fn run(path: PathBuf, args: DbgArgs) {
    {
        let path = path.clone();
        std::thread::spawn(move || unsafe {
            let mut si = STARTUPINFOW::default();
            let mut pi = PROCESS_INFORMATION::default();
            CreateProcessW(
                &HSTRING::from(path.as_path()),
                PWSTR::null(),
                None,
                None,
                false,
                DEBUG_PROCESS | DEBUG_ONLY_THIS_PROCESS,
                None,
                PCWSTR::null(),
                &mut si,
                &mut pi,
            )
            .ok()
            .unwrap();

            let mut event = DEBUG_EVENT::default();
            loop {
                WaitForDebugEvent(&mut event, INFINITE).ok().unwrap();
                match event.dwDebugEventCode {
                    CREATE_PROCESS_DEBUG_EVENT => {
                        let _event_info = event.u.CreateProcessInfo;
                        peb::hide_debugger(pi.hProcess);
                    }
                    _ => {}
                }
                ContinueDebugEvent(event.dwProcessId, event.dwThreadId, DBG_CONTINUE);
            }
        });
    }

    let mut input = String::new();
    loop {
        std::io::stdin().read_line(&mut input).unwrap();
        match DbgCommand::try_parse_from(std::iter::once("").chain(input.trim().split(' '))) {
            Ok(value) => match value {
                DbgCommand::Exit => {
                    break;
                }
                DbgCommand::Info => {
                    info::run(path.clone());
                }
                DbgCommand::Dump(dbg_args) => {
                    dump::run(path.clone(), dbg_args);
                }
            },
            Err(error) => {
                let _ = error.print();
            }
        }
        input.clear();
    }
}
