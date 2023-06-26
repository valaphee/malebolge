use std::{ffi::OsString, ops::Range, os::windows::ffi::OsStringExt};

use byteorder::{ReadBytesExt, LE};
use object::{
    pe::{ImageTlsDirectory64, IMAGE_DIRECTORY_ENTRY_TLS},
    read::pe::{ImageNtHeaders, ImageOptionalHeader, PeFile64},
    LittleEndian, Object, ReadRef,
};
use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{HANDLE, HMODULE, MAX_PATH},
        System::{
            Diagnostics::Debug::ReadProcessMemory,
            LibraryLoader::GetModuleHandleW,
            ProcessStatus::{
                EnumProcessModules, GetModuleBaseNameW, GetModuleInformation, MODULEINFO,
            },
            Threading::{
                NtQueryInformationProcess, ProcessBasicInformation, PROCESS_BASIC_INFORMATION,
            },
        },
    },
};

use crate::win::process::PEB;

pub struct Module {
    process: HANDLE,
    name: String,
    base: *mut std::ffi::c_void,
    size: usize,
}

impl Module {
    pub fn all(process: HANDLE) -> Vec<Self> {
        let mut result = vec![];
        unsafe {
            let mut modules = [HMODULE::default(); 128];
            let mut modules_length = 0;
            if EnumProcessModules(
                process,
                modules.as_mut_ptr(),
                std::mem::size_of_val(&modules) as u32,
                &mut modules_length,
            )
            .ok()
            .is_ok()
            {
                for &module in &modules[..modules_length as usize / std::mem::size_of::<HMODULE>()]
                {
                    result.push(Self::new(process, module))
                }
            } else {
                result.push(Self::new_image(process))
            }
        }
        result
    }

    pub fn by_name(process: HANDLE, name: String) -> Self {
        unsafe {
            let module = GetModuleHandleW(&HSTRING::from(name)).unwrap();
            Self::new(process, module)
        }
    }

    pub fn new(process: HANDLE, module: HMODULE) -> Self {
        unsafe {
            let mut module_name = [0; MAX_PATH as usize];
            GetModuleBaseNameW(process, module, &mut module_name);
            let module_name =
                OsString::from_wide(module_name.split(|&elem| elem == 0).next().unwrap())
                    .into_string()
                    .ok()
                    .unwrap();
            let mut module_info = MODULEINFO::default();
            GetModuleInformation(
                process,
                module,
                &mut module_info,
                std::mem::size_of_val(&module_info) as u32,
            )
            .ok()
            .unwrap();
            Self {
                process,
                name: module_name,
                base: module_info.lpBaseOfDll,
                size: module_info.SizeOfImage as usize,
            }
        }
    }

    pub fn new_image(process: HANDLE) -> Self {
        unsafe {
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
            Self {
                process,
                name: "".to_string(),
                base: peb.ImageBaseAddress,
                size: PeFile64::parse(ProcessMemoryReadRef {
                    process,
                    base: peb.ImageBaseAddress,
                })
                .unwrap()
                .nt_headers()
                .optional_header()
                .size_of_image() as usize,
            }
        }
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn base(&self) -> usize {
        self.base as usize
    }

    pub fn size(&self) -> usize {
        self.size
    }

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
