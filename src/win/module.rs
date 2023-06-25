use std::ops::Range;

use byteorder::{ReadBytesExt, LE};
use object::{
    pe::{ImageTlsDirectory64, IMAGE_DIRECTORY_ENTRY_TLS},
    read::pe::{ImageNtHeaders, ImageOptionalHeader, PeFile64},
    LittleEndian, Object, ReadRef,
};
use windows::Win32::{Foundation::HANDLE, System::Diagnostics::Debug::ReadProcessMemory};

pub struct Module {
    process: HANDLE,

    base: *mut std::ffi::c_void,
    size: usize,
}

impl Module {
    pub fn new(process: HANDLE, base: *mut std::ffi::c_void) -> Self {
        let image = PeFile64::parse(ProcessMemoryReadRef { process, base }).unwrap();
        Self {
            process,
            base,
            size: image.nt_headers().optional_header().size_of_image() as usize,
        }
    }

    pub fn base(&self) -> usize {
        self.base as usize
    }

    pub fn size(&self) -> usize { self.size as usize }

    pub fn symbols(&self) -> Vec<(String, usize)> {
        let mut data = vec![0; self.size];
        unsafe {
            ReadProcessMemory(
                self.process,
                self.base,
                data.as_mut_ptr() as *mut _,
                data.len(),
                None,
            );
        }
        let image = PeFile64::parse(data.as_slice()).unwrap();

        let mut symbols = vec![("entry_point".to_owned(), image.entry() as usize)];
        if let Some(directory) = image.data_directory(IMAGE_DIRECTORY_ENTRY_TLS) {
            if let Ok(directory_data) = directory.data(image.data(), &image.section_table()) {
                if let Ok(tls_directory) = directory_data.read_at::<ImageTlsDirectory64>(0) {
                    if let Some(mut callback_data) = image.section_table().pe_data_at(
                        image.data(),
                        (tls_directory.address_of_call_backs.get(LittleEndian)
                            - image.relative_address_base()) as u32,
                    ) {
                        let mut i = 0;
                        loop {
                            let callback = callback_data.read_u64::<LE>().unwrap();
                            if callback == 0 {
                                break;
                            }
                            symbols.push((format!("tls_callback_{}", i), callback as usize));
                            i += 1;
                        }
                    }
                }
            }
        }
        symbols
    }

    pub fn symbol(&self, name: &str) -> Option<usize> {
        let mut data = vec![0; self.size];
        unsafe {
            ReadProcessMemory(
                self.process,
                self.base,
                data.as_mut_ptr() as *mut _,
                data.len(),
                None,
            );
        }
        let image = PeFile64::parse(data.as_slice()).unwrap();

        match name {
            "entry_point" => Some(image.entry() as usize),
            name => {
                if let Some(name) = name.strip_prefix("tls_callback_") {
                    let callback_ordinal = name.parse::<usize>().unwrap();
                    let Some(directory) = image.data_directory(IMAGE_DIRECTORY_ENTRY_TLS) else {
                        return None;
                    };
                    let Ok(directory_data) = directory.data(image.data(), &image.section_table()) else {
                        return None;
                    };
                    let Ok(tls_directory) = directory_data.read_at::<ImageTlsDirectory64>(0) else {
                        return None;
                    };
                    let Some(mut callback_data) = image.section_table().pe_data_at(
                        image.data(),
                        (tls_directory.address_of_call_backs.get(LittleEndian)
                            - image.relative_address_base()) as u32,
                    ) else {
                        return None;
                    };
                    for _ in 0..callback_ordinal {
                        let callback = callback_data.read_u64::<LE>().unwrap();
                        if callback == 0 {
                            return None;
                        }
                    }
                    let callback = callback_data.read_u64::<LE>().unwrap();
                    if callback == 0 {
                        None
                    } else {
                        Some(callback as usize)
                    }
                } else {
                    None
                }
            }
        }
    }
}

#[derive(Copy, Clone)]
struct ProcessMemoryReadRef {
    process: HANDLE,
    base: *mut std::ffi::c_void,
}

impl<'a> ReadRef<'a> for ProcessMemoryReadRef {
    fn len(self) -> Result<u64, ()> {
        todo!()
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
        todo!()
    }
}
