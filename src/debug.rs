use eframe::egui;
use tokio::sync::mpsc;
use windows::Win32::{
    Foundation::{DBG_CONTINUE},
    System::{
        Diagnostics::Debug::{
            ContinueDebugEvent, DebugActiveProcess, WaitForDebugEvent,
            CREATE_PROCESS_DEBUG_EVENT, CREATE_THREAD_DEBUG_EVENT, DEBUG_EVENT,
            EXCEPTION_DEBUG_EVENT, EXIT_PROCESS_DEBUG_EVENT, EXIT_THREAD_DEBUG_EVENT,
            LOAD_DLL_DEBUG_EVENT, OUTPUT_DEBUG_STRING_EVENT, RIP_EVENT, UNLOAD_DLL_DEBUG_EVENT,
        },
        Threading::{
            INFINITE
        },
    },
};

pub struct Debugger {
    pub event_rx: mpsc::Receiver<DebugEvent>,
    pub status_tx: mpsc::Sender<DebugStatus>,
}

#[derive(Debug)]
pub enum DebugEvent {
    Exception {
        address: usize,
    }
}

#[derive(Debug)]
pub enum DebugStatus {
    Continue
}

impl Debugger {
    pub fn new(pid: u32, ctx: egui::Context) -> Self {
        let (event_tx, event_rx) = mpsc::channel(1);
        let (status_tx, mut status_rx) = mpsc::channel(1);
        std::thread::spawn(move || {
            unsafe {
                DebugActiveProcess(pid).ok().unwrap();
                loop {
                    let mut debug_event = DEBUG_EVENT::default();
                    WaitForDebugEvent(&mut debug_event, INFINITE).ok().unwrap();
                    let status = match debug_event.dwDebugEventCode {
                        EXCEPTION_DEBUG_EVENT => {
                            let debug_info = debug_event.u.Exception;
                            let event = DebugEvent::Exception {
                                address: debug_info.ExceptionRecord.ExceptionAddress.addr(),
                            };
                            pollster::block_on(async {
                                event_tx.send(event).await.unwrap();
                                ctx.request_repaint();
                                status_rx.recv().await.unwrap()
                            })
                        }
                        _ => DebugStatus::Continue,
                    };
                    ContinueDebugEvent(
                        debug_event.dwProcessId,
                        debug_event.dwThreadId,
                        match status { DebugStatus::Continue => DBG_CONTINUE },
                    )
                    .ok()
                    .unwrap();
                }
            }
        });
        Self {
            event_rx,
            status_tx
        }
    }
}
