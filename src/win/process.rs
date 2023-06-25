use std::path::Path;

use windows::{
    core::{HSTRING, PCWSTR, PWSTR},
    Win32::{
        Foundation::{HANDLE, UNICODE_STRING},
        System::{
            Diagnostics::Debug::ReadProcessMemory,
            Kernel::STRING,
            Threading::{
                CreateProcessW, NtQueryInformationProcess, ProcessBasicInformation,
                TerminateProcess, CREATE_SUSPENDED, PEB_LDR_DATA, PPS_POST_PROCESS_INIT_ROUTINE,
                PROCESS_BASIC_INFORMATION, PROCESS_INFORMATION, STARTUPINFOW,
            },
        },
    },
};
use windows::Win32::Foundation::CloseHandle;

use crate::win::module::Module;

pub struct Process {
    handle: HANDLE,

    base_module_name: String,
}

impl Process {
    pub fn new(path: impl AsRef<Path>) -> Self {
        unsafe {
            let startup_info = STARTUPINFOW::default();
            let mut process_info = PROCESS_INFORMATION::default();
            CreateProcessW(
                &HSTRING::from(path.as_ref()),
                PWSTR::null(),
                None,
                None,
                false,
                CREATE_SUSPENDED,
                None,
                PCWSTR::null(),
                &startup_info,
                &mut process_info,
            )
            .ok()
            .unwrap();


            Self {
                handle: process_info.hProcess,
                base_module_name: path.as_ref().file_name().unwrap().to_str().unwrap().to_string()
            }
        }
    }

    pub fn modules(&self) -> Vec<(String, Module)> {
        vec![(self.base_module_name.clone(), unsafe {
            let mut pbi = PROCESS_BASIC_INFORMATION::default();
            NtQueryInformationProcess(
                self.handle,
                ProcessBasicInformation,
                std::ptr::addr_of_mut!(pbi) as *mut _,
                std::mem::size_of_val(&pbi) as u32,
                &mut 0,
            )
                .unwrap();
            let mut peb = std::mem::zeroed::<PEB>();
            ReadProcessMemory(
                self.handle,
                pbi.PebBaseAddress as *mut _,
                std::ptr::addr_of_mut!(peb) as *mut _,
                std::mem::size_of_val(&peb),
                None,
            )
                .ok()
                .unwrap();
            Module::new(self.handle, peb.ImageBaseAddress)
        })]
    }

    pub fn module(&self, name: Option<String>) -> Option<Module> {
        if let Some(_name) = name {
            None
        } else {
            unsafe {
                let mut pbi = PROCESS_BASIC_INFORMATION::default();
                NtQueryInformationProcess(
                    self.handle,
                    ProcessBasicInformation,
                    std::ptr::addr_of_mut!(pbi) as *mut _,
                    std::mem::size_of_val(&pbi) as u32,
                    &mut 0,
                )
                .unwrap();
                let mut peb = std::mem::zeroed::<PEB>();
                ReadProcessMemory(
                    self.handle,
                    pbi.PebBaseAddress as *mut _,
                    std::ptr::addr_of_mut!(peb) as *mut _,
                    std::mem::size_of_val(&peb),
                    None,
                )
                .ok()
                .unwrap();
                Some(Module::new(self.handle, peb.ImageBaseAddress))
            }
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        unsafe {
            TerminateProcess(self.handle, 0).ok().unwrap();
            CloseHandle(self.handle);
        }
    }
}

#[repr(C)]
#[derive(Debug)]
#[allow(non_snake_case)]
struct PEB {
    InheritedAddressSpace: u8,
    ReadImageFileExecOptions: u8,
    BeingDebugged: u8,
    BitField: u8,
    Mutant: *mut std::ffi::c_void,
    ImageBaseAddress: *mut std::ffi::c_void,
    Ldr: *mut PEB_LDR_DATA,
    ProcessParameters: *mut RTL_USER_PROCESS_PARAMETERS,
    SubSystemData: *mut std::ffi::c_void,
    ProcessHeap: *mut std::ffi::c_void,
    FastPebLock: *mut std::ffi::c_void,
    AtlThunkSListPtr: *mut std::ffi::c_void,
    IFEOKey: *mut std::ffi::c_void,
    CrossProcessFlags: u32,
    KernelCallbackTable: *mut std::ffi::c_void,
    SystemReserved: u32,
    AtlThunkSListPtr32: u32,
    ApiSetMap: *mut std::ffi::c_void,
    TlsExpansionCounter: u32,
    TlsBitmap: *mut std::ffi::c_void,
    TlsBitmapBits: [u32; 2],
    ReadOnlySharedMemoryBase: *mut std::ffi::c_void,
    SharedData: *mut std::ffi::c_void,
    ReadOnlyStaticServerData: *mut std::ffi::c_void,
    AnsiCodePageData: *mut std::ffi::c_void,
    OemCodePageData: *mut std::ffi::c_void,
    UnicodeCaseTableData: *mut std::ffi::c_void,
    NumberOfProcessors: u32,
    NtGlobalFlag: u32,
    CriticalSectionTimeout: u64,
    HeapSegmentReserve: usize,
    HeapSegmentCommit: usize,
    HeapDeCommitTotalFreeThreshold: usize,
    HeapDeCommitFreeBlockThreshold: usize,
    NumberOfHeaps: u32,
    MaximumNumberOfHeaps: u32,
    ProcessHeaps: usize,
    GdiSharedHandleTable: *mut std::ffi::c_void,
    ProcessStarterHelper: *mut std::ffi::c_void,
    GdiDCAttributeList: u32,
    LoaderLock: *mut std::ffi::c_void,
    OSSMajorVersion: u32,
    OSMinorVersion: u32,
    OSBuildNumber: u16,
    OSCSDVersion: u16,
    OSPlatformId: u32,
    ImageSubsystem: u32,
    ImageSubsystemMajorVersion: u32,
    ImageSubsystemMinorVersion: u32,
    ActiveProcessAffinityMask: u64,
    GdiHandleBuffer: [u32; 0x3C],
    PostProcessInitRoutine: PPS_POST_PROCESS_INIT_ROUTINE,
    TlsExpansionBitmap: *mut std::ffi::c_void,
    TlsExpansionBitmapBits: [u32; 0x20],
    SessionId: u32,
}

#[repr(C)]
#[derive(Debug)]
#[allow(non_snake_case)]
struct RTL_USER_PROCESS_PARAMETERS {
    MaximumLength: u32,
    Length: u32,
    Flags: u32,
    DebugFlags: u32,
    ConsoleHandle: *mut std::ffi::c_void,
    ConsoleFlags: u32,
    StandardInput: *mut std::ffi::c_void,
    StandardOutput: *mut std::ffi::c_void,
    StandardError: *mut std::ffi::c_void,
    CurrentDirectory: CURDIR,
    DllPath: UNICODE_STRING,
    ImagePathName: UNICODE_STRING,
    CommandLine: UNICODE_STRING,
    Environment: *mut std::ffi::c_void,
    StartingX: u32,
    StartingY: u32,
    CountX: u32,
    CountY: u32,
    CountCharsX: u32,
    CountCharsY: u32,
    FillAttribute: u32,
    WindowFlags: u32,
    ShowWindowFlags: u32,
    WindowTitle: UNICODE_STRING,
    DesktopInfo: UNICODE_STRING,
    ShellInfo: UNICODE_STRING,
    RuntimeData: UNICODE_STRING,
    CurrentDirectories: [RTL_DRIVE_LETTER_CURDIR; 0x20],
    EnvironmentSize: usize,
    EnvironmentVersion: usize,
    PackageDependencyData: *mut std::ffi::c_void,
    ProcessGroupId: u32,
    LoaderThreads: u32,
    RedirectionDllName: UNICODE_STRING,
    HeapPartitionName: UNICODE_STRING,
    DefaultThreadpoolCpuSetMasks: *mut core::ffi::c_void,
    DefaultThreadpoolCpuSetMaskCount: u32,
    DefaultThreadpoolThreadMaximum: u32,
}

#[repr(C)]
#[derive(Debug)]
#[allow(non_snake_case)]
struct CURDIR {
    DosPath: UNICODE_STRING,
    Handle: *mut std::ffi::c_void,
}

#[repr(C)]
#[derive(Debug)]
#[allow(non_snake_case)]
struct RTL_DRIVE_LETTER_CURDIR {
    Flags: u16,
    Length: u16,
    TimeStamp: u32,
    DosPath: STRING,
}
