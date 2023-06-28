use windows::{
    core::PCSTR,
    s,
    Win32::{
        Foundation::{
            CloseHandle, DuplicateHandle, DUPLICATE_HANDLE_OPTIONS, DUPLICATE_SAME_ACCESS, FALSE,
            HANDLE, INVALID_HANDLE_VALUE,
        },
        System::{
            Memory::{
                CreateFileMappingA, MapViewOfFile, OpenFileMappingA, UnmapViewOfFile,
                FILE_MAP_READ, FILE_MAP_WRITE, MEMORYMAPPEDVIEW_HANDLE, PAGE_READWRITE,
            },
            Threading::{CreateEventA, GetCurrentProcess},
        },
    },
};

pub struct BreakpointListOwner {
    pub trigger_event: HANDLE,
    pub resolve_event: HANDLE,
    pub file: HANDLE,
    pub data: &'static mut BreakpointList,
}

impl BreakpointListOwner {
    pub fn new(target_process: HANDLE) -> Self {
        unsafe {
            // create new file mapping with the name BreakpointList and map it
            let file = CreateFileMappingA(
                INVALID_HANDLE_VALUE,
                None,
                PAGE_READWRITE,
                0,
                std::mem::size_of::<BreakpointList>() as u32,
                s!("BreakpointList"),
            )
            .unwrap();
            let data = &mut *(MapViewOfFile(
                file,
                FILE_MAP_READ | FILE_MAP_WRITE,
                0,
                0,
                std::mem::size_of::<BreakpointList>(),
            )
            .unwrap()
            .0 as *mut BreakpointList);

            // create trigger and resolve events which are used for IPC
            let process = GetCurrentProcess();
            let trigger_event = CreateEventA(None, FALSE, FALSE, PCSTR::null()).unwrap();
            let mut target_trigger_event = HANDLE::default();
            DuplicateHandle(
                process,
                trigger_event,
                target_process,
                &mut target_trigger_event,
                DUPLICATE_SAME_ACCESS.0,
                FALSE,
                DUPLICATE_HANDLE_OPTIONS::default(),
            )
            .ok()
            .unwrap();
            let resolve_event = CreateEventA(None, FALSE, FALSE, PCSTR::null()).unwrap();
            let mut target_resolve_event = HANDLE::default();
            DuplicateHandle(
                process,
                resolve_event,
                target_process,
                &mut target_resolve_event,
                DUPLICATE_SAME_ACCESS.0,
                FALSE,
                DUPLICATE_HANDLE_OPTIONS::default(),
            )
            .ok()
            .unwrap();

            // store the duplicated events into the shared memory
            *data = BreakpointList {
                trigger_event: target_trigger_event,
                resolve_event: target_resolve_event,
                entries: [BreakpointEntry::default(); 256],
            };
            Self {
                trigger_event,
                resolve_event,
                file,
                data,
            }
        }
    }
}

impl Drop for BreakpointListOwner {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(MEMORYMAPPEDVIEW_HANDLE(self.data as *mut _ as isize));
            CloseHandle(self.file).ok().unwrap();
        }
    }
}

pub struct BreakpointList {
    pub trigger_event: HANDLE,
    pub resolve_event: HANDLE,
    pub entries: [BreakpointEntry; 256],
}

impl BreakpointList {
    pub fn new() -> &'static mut BreakpointList {
        unsafe {
            let file = OpenFileMappingA(
                (FILE_MAP_READ | FILE_MAP_WRITE).0,
                FALSE,
                s!("BreakpointList"),
            )
            .unwrap();
            &mut *(MapViewOfFile(
                file,
                FILE_MAP_READ | FILE_MAP_WRITE,
                0,
                0,
                std::mem::size_of::<BreakpointList>(),
            )
            .unwrap()
            .0 as *mut BreakpointList)
        }
    }
}

#[derive(Copy, Clone)]
pub struct BreakpointEntry {
    pub address: usize,
    pub trigger: bool,
}

impl Default for BreakpointEntry {
    fn default() -> Self {
        Self {
            address: 0,
            trigger: false,
        }
    }
}
