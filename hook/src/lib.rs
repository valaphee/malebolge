use windows::Win32::{
    Foundation::HMODULE,
    System::{
        Diagnostics::Debug::{AddVectoredExceptionHandler, EXCEPTION_POINTERS},
        LibraryLoader::DisableThreadLibraryCalls,
        SystemServices::DLL_PROCESS_ATTACH,
        Threading::{SetEvent, WaitForSingleObject, INFINITE},
    },
};

use mbg_hook_shared::{BreakpointList, BreakpointListView};

static mut BREAKPOINT_LIST: *mut BreakpointList = std::ptr::null_mut::<BreakpointList>();

#[no_mangle]
unsafe extern "system" fn DllMain(
    module: HMODULE,
    reason: u32,
    _reserved: *const std::ffi::c_void,
) -> bool {
    if reason == DLL_PROCESS_ATTACH {
        DisableThreadLibraryCalls(module).ok().unwrap();

        // open the shared memory wrapper, and leak it
        let mut breakpoint_list_view = BreakpointListView::open();
        BREAKPOINT_LIST = breakpoint_list_view.data();
        std::mem::forget(breakpoint_list_view);

        AddVectoredExceptionHandler(1, Some(vectored_exception_handler));
    }

    true
}

const EXCEPTION_CONTINUE_EXECUTION: i32 = -1;
const EXCEPTION_CONTINUE_SEARCH: i32 = 0;
const _EXCEPTION_EXECUTE_HANDLER: i32 = 1;

unsafe extern "system" fn vectored_exception_handler(
    exception_pointers: *mut EXCEPTION_POINTERS,
) -> i32 {
    let exception_pointers = *exception_pointers;
    let exception = *exception_pointers.ExceptionRecord;
    /* let context = *exception_pointers.ContextRecord; */

    // find and trigger the breakpoint
    let breakpoint_list = &mut *BREAKPOINT_LIST;
    let Some(breakpoint) = breakpoint_list.entries.iter_mut().find(|entry| entry.address == exception.ExceptionAddress as usize) else {
        return EXCEPTION_CONTINUE_SEARCH;
    };
    breakpoint.trigger = true;

    // notify the host process and wait until the trigger has been reset
    SetEvent(breakpoint_list.chld_event_dup).ok().unwrap();
    loop {
        WaitForSingleObject(breakpoint_list.prnt_event_dup, INFINITE)
            .ok()
            .unwrap();
        if !breakpoint.trigger {
            break;
        }
    }

    EXCEPTION_CONTINUE_EXECUTION
}
