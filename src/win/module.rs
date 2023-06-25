use std::ops::Range;

use object::ReadRef;
use windows::Win32::{Foundation::HANDLE, System::Diagnostics::Debug::ReadProcessMemory};

#[derive(Copy, Clone)]
pub struct ModuleView {
    process: HANDLE,
    base: *mut std::ffi::c_void,
}

impl ModuleView {
    pub fn new(process: HANDLE, base: *mut std::ffi::c_void) -> Self {
        Self { process, base }
    }
}

impl<'a> ReadRef<'a> for ModuleView {
    fn len(self) -> Result<u64, ()> {
        unimplemented!()
    }

    fn read_bytes_at(self, offset: u64, size: u64) -> Result<&'a [u8], ()> {
        let mut value = vec![0u8; size as usize];
        unsafe {
            ReadProcessMemory(
                self.process,
                self.base.add(offset as usize),
                value.as_mut_ptr() as *mut _,
                size as usize,
                None,
            );
        }
        Ok(value.leak())
    }

    fn read_bytes_at_until(self, _range: Range<u64>, _delimiter: u8) -> Result<&'a [u8], ()> {
        unimplemented!()
    }
}
