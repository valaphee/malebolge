use std::path::Path;

use windows::{
    core::{HSTRING, PCWSTR, PWSTR},
    s,
    Win32::{
        Foundation::{CloseHandle, FALSE, HANDLE, UNICODE_STRING},
        System::{
            Diagnostics::Debug::WriteProcessMemory,
            Kernel::STRING,
            LibraryLoader::{GetModuleHandleA, GetProcAddress},
            Memory::{VirtualAllocEx, MEM_COMMIT, MEM_RESERVE, PAGE_READWRITE},
            Threading::{
                CreateProcessW, CreateRemoteThread, ResumeThread, TerminateProcess,
                WaitForSingleObject, CREATE_SUSPENDED, INFINITE, PEB_LDR_DATA,
                PPS_POST_PROCESS_INIT_ROUTINE, PROCESS_INFORMATION, STARTUPINFOW,
                THREAD_CREATE_RUN_IMMEDIATELY,
            },
        },
    },
};

use crate::win::module::Module;

pub struct Process {
    process: HANDLE,
    thread: HANDLE,

    name: String,
}

impl Process {
    /// spawns a new process
    pub fn new(path: impl AsRef<Path>) -> Self {
        unsafe {
            let startup_info = STARTUPINFOW::default();
            let mut process_info = PROCESS_INFORMATION::default();
            CreateProcessW(
                &HSTRING::from(path.as_ref()),
                PWSTR::null(),
                None,
                None,
                FALSE,
                CREATE_SUSPENDED,
                None,
                PCWSTR::null(),
                &startup_info,
                &mut process_info,
            )
            .ok()
            .unwrap();
            Self {
                process: process_info.hProcess,
                thread: process_info.hThread,
                name: path
                    .as_ref()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
            }
        }
    }

    /// all known modules
    pub fn modules(&self) -> Vec<Module> {
        Module::all(self.process)
    }

    /// searches for a module with the specified name, if the name is None the
    /// image module will be returned
    pub fn module(&self, name: Option<String>) -> Option<Module> {
        Some(Module::by_name(
            self.process,
            name.unwrap_or(self.name.clone()),
        ))
    }

    /// loads a library into the process
    pub fn load_library(&self, path: impl AsRef<Path>) -> windows::core::Result<()> {
        let path = HSTRING::from(path.as_ref());
        let path = path.as_wide();
        unsafe {
            let path_address = VirtualAllocEx(
                self.process,
                None,
                std::mem::size_of_val(path),
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            );
            WriteProcessMemory(
                self.process,
                path_address as *const std::ffi::c_void,
                path.as_ptr() as *const _,
                std::mem::size_of_val(path),
                None,
            )
            .ok()?;
            let load_library_w =
                GetProcAddress(GetModuleHandleA(s!("kernel32.dll"))?, s!("LoadLibraryW"));
            let load_library_thread = CreateRemoteThread(
                self.process,
                None,
                0,
                Some(std::mem::transmute(load_library_w)),
                Some(path_address),
                THREAD_CREATE_RUN_IMMEDIATELY.0,
                None,
            )?;
            WaitForSingleObject(load_library_thread, INFINITE).ok()
        }
    }

    pub fn resume(&self) {
        unsafe {
            ResumeThread(self.thread);
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            TerminateProcess(self.process, 0).ok().unwrap();
            CloseHandle(self.process);
        }
    }
}

#[repr(C)]
#[derive(Debug)]
#[allow(non_snake_case)]
pub struct PEB {
    pub InheritedAddressSpace: u8,
    pub ReadImageFileExecOptions: u8,
    pub BeingDebugged: u8,
    pub BitField: u8,
    pub Mutant: *mut std::ffi::c_void,
    pub ImageBaseAddress: *mut std::ffi::c_void,
    pub Ldr: *mut PEB_LDR_DATA,
    pub ProcessParameters: *mut RTL_USER_PROCESS_PARAMETERS,
    pub SubSystemData: *mut std::ffi::c_void,
    pub ProcessHeap: *mut std::ffi::c_void,
    pub FastPebLock: *mut std::ffi::c_void,
    pub AtlThunkSListPtr: *mut std::ffi::c_void,
    pub IFEOKey: *mut std::ffi::c_void,
    pub CrossProcessFlags: u32,
    pub KernelCallbackTable: *mut std::ffi::c_void,
    pub SystemReserved: u32,
    pub AtlThunkSListPtr32: u32,
    pub ApiSetMap: *mut std::ffi::c_void,
    pub TlsExpansionCounter: u32,
    pub TlsBitmap: *mut std::ffi::c_void,
    pub TlsBitmapBits: [u32; 2],
    pub ReadOnlySharedMemoryBase: *mut std::ffi::c_void,
    pub SharedData: *mut std::ffi::c_void,
    pub ReadOnlyStaticServerData: *mut std::ffi::c_void,
    pub AnsiCodePageData: *mut std::ffi::c_void,
    pub OemCodePageData: *mut std::ffi::c_void,
    pub UnicodeCaseTableData: *mut std::ffi::c_void,
    pub NumberOfProcessors: u32,
    pub NtGlobalFlag: u32,
    pub CriticalSectionTimeout: u64,
    pub HeapSegmentReserve: usize,
    pub HeapSegmentCommit: usize,
    pub HeapDeCommitTotalFreeThreshold: usize,
    pub HeapDeCommitFreeBlockThreshold: usize,
    pub NumberOfHeaps: u32,
    pub MaximumNumberOfHeaps: u32,
    pub ProcessHeaps: usize,
    pub GdiSharedHandleTable: *mut std::ffi::c_void,
    pub ProcessStarterHelper: *mut std::ffi::c_void,
    pub GdiDCAttributeList: u32,
    pub LoaderLock: *mut std::ffi::c_void,
    pub OSSMajorVersion: u32,
    pub OSMinorVersion: u32,
    pub OSBuildNumber: u16,
    pub OSCSDVersion: u16,
    pub OSPlatformId: u32,
    pub ImageSubsystem: u32,
    pub ImageSubsystemMajorVersion: u32,
    pub ImageSubsystemMinorVersion: u32,
    pub ActiveProcessAffinityMask: u64,
    pub GdiHandleBuffer: [u32; 0x3C],
    pub PostProcessInitRoutine: PPS_POST_PROCESS_INIT_ROUTINE,
    pub TlsExpansionBitmap: *mut std::ffi::c_void,
    pub TlsExpansionBitmapBits: [u32; 0x20],
    pub SessionId: u32,
}

#[repr(C)]
#[derive(Debug)]
#[allow(non_snake_case)]
pub struct RTL_USER_PROCESS_PARAMETERS {
    pub MaximumLength: u32,
    pub Length: u32,
    pub Flags: u32,
    pub DebugFlags: u32,
    pub ConsoleHandle: *mut std::ffi::c_void,
    pub ConsoleFlags: u32,
    pub StandardInput: *mut std::ffi::c_void,
    pub StandardOutput: *mut std::ffi::c_void,
    pub StandardError: *mut std::ffi::c_void,
    pub CurrentDirectory: CURDIR,
    pub DllPath: UNICODE_STRING,
    pub ImagePathName: UNICODE_STRING,
    pub CommandLine: UNICODE_STRING,
    pub Environment: *mut std::ffi::c_void,
    pub StartingX: u32,
    pub StartingY: u32,
    pub CountX: u32,
    pub CountY: u32,
    pub CountCharsX: u32,
    pub CountCharsY: u32,
    pub FillAttribute: u32,
    pub WindowFlags: u32,
    pub ShowWindowFlags: u32,
    pub WindowTitle: UNICODE_STRING,
    pub DesktopInfo: UNICODE_STRING,
    pub ShellInfo: UNICODE_STRING,
    pub RuntimeData: UNICODE_STRING,
    pub CurrentDirectories: [RTL_DRIVE_LETTER_CURDIR; 0x20],
    pub EnvironmentSize: usize,
    pub EnvironmentVersion: usize,
    pub PackageDependencyData: *mut std::ffi::c_void,
    pub ProcessGroupId: u32,
    pub LoaderThreads: u32,
    pub RedirectionDllName: UNICODE_STRING,
    pub HeapPartitionName: UNICODE_STRING,
    pub DefaultThreadpoolCpuSetMasks: *mut core::ffi::c_void,
    pub DefaultThreadpoolCpuSetMaskCount: u32,
    pub DefaultThreadpoolThreadMaximum: u32,
}

#[repr(C)]
#[derive(Debug)]
#[allow(non_snake_case)]
pub struct CURDIR {
    pub DosPath: UNICODE_STRING,
    pub Handle: *mut std::ffi::c_void,
}

#[repr(C)]
#[derive(Debug)]
#[allow(non_snake_case)]
pub struct RTL_DRIVE_LETTER_CURDIR {
    pub Flags: u16,
    pub Length: u16,
    pub TimeStamp: u32,
    pub DosPath: STRING,
}
