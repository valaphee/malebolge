use windows::{
    core::{IntoParam, Param, HSTRING, PCWSTR},
    s, w,
    Win32::{
        Foundation::HANDLE,
        System::{
            Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory},
            LibraryLoader::{GetModuleHandleW, GetProcAddress},
            Memory::{
                VirtualAllocEx, MEM_COMMIT, MEM_RESERVE, PAGE_EXECUTE_READWRITE, PAGE_READWRITE,
            },
            Threading::{
                CreateRemoteThread, WaitForSingleObject, INFINITE, THREAD_CREATE_RUN_IMMEDIATELY,
            },
        },
    },
};

pub unsafe fn load_library(process: HANDLE, library_path: &str) -> windows::core::Result<()> {
    let library_path = HSTRING::from(library_path);
    let library_path = library_path.as_wide();
    let library_path_address = VirtualAllocEx(
        process,
        None,
        std::mem::size_of_val(library_path),
        MEM_COMMIT | MEM_RESERVE,
        PAGE_EXECUTE_READWRITE,
    );
    WriteProcessMemory(
        process,
        library_path_address as *const std::ffi::c_void,
        library_path.as_ptr() as *const _,
        std::mem::size_of_val(library_path),
        None,
    )
    .ok()?;
    let load_library_w = GetProcAddress(GetModuleHandleW(w!("kernel32.dll"))?, s!("LoadLibraryW"));
    let load_library_thread = CreateRemoteThread(
        process,
        None,
        0,
        Some(std::mem::transmute(load_library_w)),
        Some(library_path_address),
        THREAD_CREATE_RUN_IMMEDIATELY.0,
        None,
    )?;
    WaitForSingleObject(load_library_thread, INFINITE).ok()?;
    Ok(())
}
