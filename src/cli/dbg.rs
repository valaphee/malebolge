use std::{net::UdpSocket, path::PathBuf, time::Duration};

use clap::{Args, Parser};
use windows::{
    core::{HSTRING, PCWSTR, PWSTR},
    Win32::System::{
        Diagnostics::Debug::RemoveVectoredExceptionHandler,
        Memory::{VirtualAllocEx, MEM_COMMIT, PAGE_EXECUTE_READWRITE},
        Threading::{
            CreateProcessW, CreateRemoteThread, OpenProcess, ResumeThread, CREATE_SUSPENDED,
            PROCESS_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
            STARTUPINFOW, THREAD_CREATE_SUSPENDED,
        },
    },
};

use crate::{
    cli::{dump, dump::DumpArgs},
    dbg::load_library,
};

#[derive(Args)]
pub struct DbgArgs {
    pid: Option<u32>,
}

#[derive(Parser)]
pub enum DbgCommand {
    Quit,
    Dump(DumpArgs),
}

#[allow(mutable_transmutes)]
pub(super) fn run(path: PathBuf, args: DbgArgs) {
    let process;
    unsafe {
        if let Some(pid) = args.pid {
            process = OpenProcess(
                PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
                false,
                pid,
            )
            .unwrap();
        } else {
            let mut si = STARTUPINFOW::default();
            let mut pi = PROCESS_INFORMATION::default();
            CreateProcessW(
                &HSTRING::from(path.as_path()),
                PWSTR::null(),
                None,
                None,
                false,
                CREATE_SUSPENDED,
                None,
                PCWSTR::null(),
                &mut si,
                &mut pi,
            )
            .ok()
            .unwrap();
            process = pi.hProcess;

            let socket = UdpSocket::bind("127.0.0.1:13371").unwrap();

            load_library(
                pi.hProcess,
                "C:\\Users\\valaphee\\CLionProjects\\malebolge\\target\\debug\\mbg.dll",
            )
            .unwrap();

            ResumeThread(pi.hThread);

            loop {
                let mut buffer = [0u8; 256];
                socket.recv(&mut buffer).unwrap();
                println!("{:?}", buffer);
            }
        };
    }

    let mut input = String::new();
    loop {
        std::io::stdin().read_line(&mut input).unwrap();
        match DbgCommand::try_parse_from(std::iter::once("").chain(input.trim().split(' '))) {
            Ok(value) => match value {
                DbgCommand::Quit => {
                    break;
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
