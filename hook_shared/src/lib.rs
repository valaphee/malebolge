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

pub struct BreakpointListView {
    file: HANDLE,

    data: &'static mut BreakpointList,
}

impl BreakpointListView {
    pub fn new(dup_process: HANDLE) -> Self {
        unsafe {
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
            *data = BreakpointList::new(dup_process);
            Self { file, data }
        }
    }

    pub fn open() -> Self {
        unsafe {
            let file = OpenFileMappingA(
                (FILE_MAP_READ | FILE_MAP_WRITE).0,
                FALSE,
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
            Self { file, data }
        }
    }

    pub fn data(&mut self) -> &mut BreakpointList {
        self.data
    }
}

impl Drop for BreakpointListView {
    fn drop(&mut self) {
        unsafe {
            UnmapViewOfFile(MEMORYMAPPEDVIEW_HANDLE(self.data as *mut _ as isize));
            CloseHandle(self.file).ok().unwrap();
        }
    }
}

pub struct BreakpointList {
    pub chld_event: HANDLE,
    pub chld_event_dup: HANDLE,
    pub prnt_event: HANDLE,
    pub prnt_event_dup: HANDLE,

    pub entries: [BreakpointEntry; 256],
}

impl BreakpointList {
    pub fn new(dup_process: HANDLE) -> Self {
        unsafe {
            let process = GetCurrentProcess();
            let chld_event = CreateEventA(None, FALSE, FALSE, PCSTR::null()).unwrap();
            let mut chld_event_dup = HANDLE::default();
            DuplicateHandle(
                process,
                chld_event,
                dup_process,
                &mut chld_event_dup,
                DUPLICATE_SAME_ACCESS.0,
                FALSE,
                DUPLICATE_HANDLE_OPTIONS::default(),
            )
            .ok()
            .unwrap();
            let prnt_event = CreateEventA(None, FALSE, FALSE, PCSTR::null()).unwrap();
            let mut prnt_event_dup = HANDLE::default();
            DuplicateHandle(
                process,
                prnt_event,
                dup_process,
                &mut prnt_event_dup,
                DUPLICATE_SAME_ACCESS.0,
                FALSE,
                DUPLICATE_HANDLE_OPTIONS::default(),
            )
            .ok()
            .unwrap();
            Self {
                chld_event,
                chld_event_dup,
                prnt_event,
                prnt_event_dup,
                entries: [BreakpointEntry::default(); 256],
            }
        }
    }
}

impl Drop for BreakpointList {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.prnt_event);
            CloseHandle(self.chld_event);
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
