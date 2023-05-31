#![feature(const_mut_refs)]

use std::{net::UdpSocket, sync::Mutex};

use windows::{
    s,
    Win32::{
        Foundation::{HMODULE, HWND},
        System::{
            Diagnostics::Debug::{AddVectoredExceptionHandler, EXCEPTION_POINTERS},
            LibraryLoader::DisableThreadLibraryCalls,
            SystemServices::DLL_PROCESS_ATTACH,
        },
        UI::WindowsAndMessaging::{MessageBoxA, MB_OK},
    },
};

#[no_mangle]
unsafe extern "system" fn DllMain(
    module: HMODULE,
    reason: u32,
    _reserved: *const std::ffi::c_void,
) -> bool {
    match reason {
        DLL_PROCESS_ATTACH => {
            DisableThreadLibraryCalls(module).ok().unwrap();
        }
        _ => {}
    }

    true
}
