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

use crate::{process::PEB, Result};

pub struct Module {
    process: HANDLE,

    name: String,
    base: usize,
    size: usize,
}

impl Module {
    /// all known modules
    pub fn all(process: HANDLE) -> Result<Vec<Self>> {
        let mut result = vec![];
        unsafe {
            let mut modules = [HMODULE::default(); 128];
            let mut modules_length = 0;
            EnumProcessModules(
                process,
                modules.as_mut_ptr(),
                std::mem::size_of_val(&modules) as u32,
                &mut modules_length,
            )
            .ok()?;
            for &module in &modules[..modules_length as usize / std::mem::size_of::<HMODULE>()] {
                result.push(Self::from_handle(process, module)?)
            }
        }
        Ok(result)
    }

    /// searches for a module with the specified name, if the name is None the
    /// image module will be returned
    pub fn by_name(process: HANDLE, name: String) -> Result<Self> {
        unsafe { Self::from_handle(process, GetModuleHandleW(&HSTRING::from(name))?) }
    }

    /// module from PEB
    pub fn from_peb(process: HANDLE) -> Result<Self> {
        unsafe {
            let mut pbi = PROCESS_BASIC_INFORMATION::default();
            NtQueryInformationProcess(
                process,
                ProcessBasicInformation,
                std::ptr::addr_of_mut!(pbi) as *mut _,
                std::mem::size_of_val(&pbi) as u32,
                &mut 0,
            )?;
            let mut peb = std::mem::zeroed::<PEB>();
            ReadProcessMemory(
                process,
                pbi.PebBaseAddress as *mut _,
                std::ptr::addr_of_mut!(peb) as *mut _,
                std::mem::size_of_val(&peb),
                None,
            )
            .ok()?;
            Ok(Self {
                process,
                name: "".to_string(),
                base: peb.ImageBaseAddress as usize,
                size: PeFile64::parse(ProcessMemoryReadRef {
                    process,
                    base: peb.ImageBaseAddress,
                })?
                .nt_headers()
                .optional_header()
                .size_of_image() as usize,
            })
        }
    }

    /// module from handle
    pub fn from_handle(process: HANDLE, module: HMODULE) -> Result<Self> {
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
            .ok()?;
            Ok(Self {
                process,
                name: module_name,
                base: module_info.lpBaseOfDll as usize,
                size: module_info.SizeOfImage as usize,
            })
        }
    }

    /// name of the module
    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    /// base address of the module
    pub fn base(&self) -> usize {
        self.base as usize
    }

    /// size of the module
    pub fn size(&self) -> usize {
        self.size
    }

    /// all known addresses of the module
    pub fn symbols(&self) -> Result<Vec<(String, usize)>> {
        let mut data = vec![0; self.size];
        unsafe {
            ReadProcessMemory(
                self.process,
                self.base as *const std::ffi::c_void,
                data.as_mut_ptr() as *mut _,
                data.len(),
                None,
            )
            .ok()?;
        }
        let image = PeFile64::parse(data.as_slice())?;

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
                            let callback = callback_data.read_u64::<LE>()?;
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
        Ok(symbols)
    }

    /// searches for an address with the specified name
    pub fn symbol(&self, name: &str) -> Result<Option<usize>> {
        let mut data = vec![0; self.size];
        unsafe {
            ReadProcessMemory(
                self.process,
                self.base as *const std::ffi::c_void,
                data.as_mut_ptr() as *mut _,
                data.len(),
                None,
            )
            .ok()?;
        }
        let image = PeFile64::parse(data.as_slice())?;

        match name {
            "entry_point" => Ok(Some(image.entry() as usize)),
            name => {
                if let Some(name) = name.strip_prefix("tls_callback_") {
                    let callback_ordinal = name.parse::<usize>().unwrap();
                    let Some(directory) = image.data_directory(IMAGE_DIRECTORY_ENTRY_TLS) else {
                        return Ok(None);
                    };
                    let Ok(directory_data) = directory.data(image.data(), &image.section_table()) else {
                        return Ok(None);
                    };
                    let Ok(tls_directory) = directory_data.read_at::<ImageTlsDirectory64>(0) else {
                        return Ok(None);
                    };
                    let Some(mut callback_data) = image.section_table().pe_data_at(
                        image.data(),
                        (tls_directory.address_of_call_backs.get(LittleEndian)
                            - image.relative_address_base()) as u32,
                    ) else {
                        return Ok(None);
                    };
                    for _ in 0..callback_ordinal {
                        let callback = callback_data.read_u64::<LE>()?;
                        if callback == 0 {
                            return Ok(None);
                        }
                    }
                    let callback = callback_data.read_u64::<LE>()?;
                    if callback == 0 {
                        Ok(None)
                    } else {
                        Ok(Some(callback as usize))
                    }
                } else {
                    Ok(None)
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
    fn len(self) -> std::result::Result<u64, ()> {
        todo!()
    }

    fn read_bytes_at(self, offset: u64, size: u64) -> std::result::Result<&'a [u8], ()> {
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

    fn read_bytes_at_until(
        self,
        _range: Range<u64>,
        _delimiter: u8,
    ) -> std::result::Result<&'a [u8], ()> {
        todo!()
    }
}
