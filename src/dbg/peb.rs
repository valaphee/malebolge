use windows::Win32::{
    Foundation::{HANDLE, UNICODE_STRING},
    System::{
        Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory},
        Kernel::STRING,
        Threading::{
            NtQueryInformationProcess, ProcessBasicInformation, PEB_LDR_DATA,
            PPS_POST_PROCESS_INIT_ROUTINE, PROCESS_BASIC_INFORMATION,
        },
    },
};

pub unsafe fn hide_debugger(process: HANDLE) {
    let mut pbi = PROCESS_BASIC_INFORMATION::default();
    NtQueryInformationProcess(
        process,
        ProcessBasicInformation,
        std::ptr::addr_of_mut!(pbi) as *mut _,
        std::mem::size_of_val(&pbi) as u32,
        &mut 0,
    )
    .unwrap();

    let mut peb = std::mem::zeroed::<PEB>();
    ReadProcessMemory(
        process,
        pbi.PebBaseAddress as *mut _,
        std::ptr::addr_of_mut!(peb) as *mut _,
        std::mem::size_of_val(&peb),
        None,
    )
    .ok()
    .unwrap();

    let mut peb_pp = std::mem::zeroed::<RTL_USER_PROCESS_PARAMETERS>();
    ReadProcessMemory(
        process,
        peb.ProcessParameters as *mut _,
        std::ptr::addr_of_mut!(peb_pp) as *mut _,
        std::mem::size_of_val(&peb_pp),
        None,
    )
    .ok()
    .unwrap();

    peb.BeingDebugged = 0;
    peb_pp.Flags |= 0x4000;

    WriteProcessMemory(
        process,
        peb.ProcessParameters as *mut _,
        std::ptr::addr_of_mut!(peb_pp) as *mut _,
        std::mem::size_of_val(&peb_pp),
        None,
    )
    .ok()
    .unwrap();
    WriteProcessMemory(
        process,
        pbi.PebBaseAddress as *mut _,
        std::ptr::addr_of_mut!(peb) as *mut _,
        std::mem::size_of_val(&peb),
        None,
    )
    .ok()
    .unwrap();
}

#[repr(C)]
#[derive(Debug)]
#[allow(non_snake_case)]
struct PEB {
    InheritedAddressSpace: u8,
    ReadImageFileExecOptions: u8,
    BeingDebugged: u8,
    BitField: u8,
    Mutant: *mut ::core::ffi::c_void,
    ImageBaseAddress: *mut ::core::ffi::c_void,
    Ldr: *mut PEB_LDR_DATA,
    ProcessParameters: *mut RTL_USER_PROCESS_PARAMETERS,
    SubSystemData: *mut ::core::ffi::c_void,
    ProcessHeap: *mut ::core::ffi::c_void,
    FastPebLock: *mut ::core::ffi::c_void,
    AtlThunkSListPtr: *mut ::core::ffi::c_void,
    IFEOKey: *mut ::core::ffi::c_void,
    CrossProcessFlags: u32,
    KernelCallbackTable: *mut ::core::ffi::c_void,
    SystemReserved: u32,
    AtlThunkSListPtr32: u32,
    ApiSetMap: *mut ::core::ffi::c_void,
    TlsExpansionCounter: u32,
    TlsBitmap: *mut ::core::ffi::c_void,
    TlsBitmapBits: [u32; 2],
    ReadOnlySharedMemoryBase: *mut ::core::ffi::c_void,
    SharedData: *mut ::core::ffi::c_void,
    ReadOnlyStaticServerData: *mut ::core::ffi::c_void,
    AnsiCodePageData: *mut ::core::ffi::c_void,
    OemCodePageData: *mut ::core::ffi::c_void,
    UnicodeCaseTableData: *mut ::core::ffi::c_void,
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
    GdiSharedHandleTable: *mut ::core::ffi::c_void,
    ProcessStarterHelper: *mut ::core::ffi::c_void,
    GdiDCAttributeList: u32,
    LoaderLock: *mut ::core::ffi::c_void,
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
    TlsExpansionBitmap: *mut ::core::ffi::c_void,
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
    ConsoleHandle: *mut ::core::ffi::c_void,
    ConsoleFlags: u32,
    StandardInput: *mut ::core::ffi::c_void,
    StandardOutput: *mut ::core::ffi::c_void,
    StandardError: *mut ::core::ffi::c_void,
    CurrentDirectory: CURDIR,
    DllPath: UNICODE_STRING,
    ImagePathName: UNICODE_STRING,
    CommandLine: UNICODE_STRING,
    Environment: *mut ::core::ffi::c_void,
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
    PackageDependencyData: *mut ::core::ffi::c_void,
    ProcessGroupId: u32,
    LoaderThreads: u32,
    RedirectionDllName: UNICODE_STRING,
    HeapPartitionName: UNICODE_STRING,
    DefaultThreadpoolCpuSetMasks: *mut ::core::ffi::c_void,
    DefaultThreadpoolCpuSetMaskCount: u32,
    DefaultThreadpoolThreadMaximum: u32,
}

#[repr(C)]
#[derive(Debug)]
#[allow(non_snake_case)]
struct CURDIR {
    DosPath: UNICODE_STRING,
    Handle: *mut ::core::ffi::c_void,
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
