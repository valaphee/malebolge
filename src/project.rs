use std::{collections::BTreeMap, path::Path};

use byteorder::{ReadBytesExt, LE};
use object::{
    pe::{
        ImageTlsDirectory64, IMAGE_DIRECTORY_ENTRY_TLS, IMAGE_SCN_CNT_CODE, IMAGE_SCN_MEM_EXECUTE,
    },
    read::pe::{ExportTarget, ImageNtHeaders, ImageOptionalHeader, PeFile64},
    LittleEndian, Object, ReadRef,
};
use thiserror::Error;
use windows::Win32::{
    Foundation::{CloseHandle, FALSE, HMODULE},
    System::{
        Diagnostics::Debug::ReadProcessMemory,
        ProcessStatus::{EnumProcessModules, GetModuleInformation, MODULEINFO},
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    },
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("Windows error")]
    Windows(#[from] windows::core::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone)]
pub struct Label {
    pub type_: LabelType,
    pub name: String,
}

#[derive(Clone)]
pub enum LabelType {
    EntryPoint,
    Export,
    TlsCallback,
    Custom,
}

pub struct Section {
    pub type_: SectionType,
    pub data_offset: usize,
    pub data_length: usize,
}

pub enum SectionType {
    Raw,
    Assembly,
}

pub struct Project {
    pub va_space: bool,
    pub data: Vec<u8>,
    pub labels: BTreeMap<u64, Label>,
}

impl Project {
    pub fn create_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let data = std::fs::read(path)?;
        let mut project = Self {
            va_space: false,
            data,
            labels: Default::default(),
        };
        project.refresh();
        Ok(project)
    }

    pub fn create_from_process(pid: u32) -> Result<Self> {
        let data = unsafe {
            let process = OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, FALSE, pid)?;
            let mut module = HMODULE::default();
            EnumProcessModules(
                process,
                &mut module,
                std::mem::size_of_val(&module) as u32,
                &mut 0,
            )
            .ok()?;
            let mut module_info = MODULEINFO::default();
            GetModuleInformation(
                process,
                module,
                &mut module_info,
                std::mem::size_of_val(&module_info) as u32,
            )
            .ok()?;
            let mut data = vec![0; module_info.SizeOfImage as usize];
            ReadProcessMemory(
                process,
                module_info.lpBaseOfDll,
                data.as_mut_ptr() as *mut std::ffi::c_void,
                data.len(),
                None,
            )
            .ok()?;
            CloseHandle(process);
            data
        };
        let mut project = Self {
            va_space: true,
            data,
            labels: Default::default(),
        };
        project.refresh();
        Ok(project)
    }

    pub fn refresh(&mut self) {
        let file = PeFile64::parse(self.data.as_slice(), self.va_space).unwrap();
        if file.nt_headers().optional_header().address_of_entry_point() != 0 {
            self.labels.insert(
                file.nt_headers().optional_header().address_of_entry_point() as u64
                    + file.relative_address_base(),
                Label {
                    type_: LabelType::EntryPoint,
                    name: "Entry point".to_string(),
                },
            );
        }
        if let Some(export_table) = file.export_table().unwrap() {
            for export in export_table.exports().unwrap() {
                match export.target {
                    ExportTarget::Address(relative_address) => {
                        self.labels.insert(
                            relative_address as u64 + file.relative_address_base(),
                            Label {
                                type_: LabelType::Export,
                                name: String::from_utf8_lossy(export.name.unwrap()).to_string(),
                            },
                        );
                    }
                    _ => {}
                }
            }
        }
        if let Some(directory) = file.data_directory(IMAGE_DIRECTORY_ENTRY_TLS) {
            if let Ok(directory_data) = directory.data(file.data(), &file.section_table()) {
                if let Ok(tls_directory) = directory_data.read_at::<ImageTlsDirectory64>(0) {
                    if let Some(mut callback_data) = file.section_table().pe_data_at(
                        file.data(),
                        (tls_directory.address_of_call_backs.get(LittleEndian)
                            - file.relative_address_base()) as u32,
                    ) {
                        loop {
                            let callback = callback_data.read_u64::<LE>().unwrap();
                            if callback == 0 {
                                break;
                            }
                            self.labels.insert(
                                callback,
                                Label {
                                    type_: LabelType::TlsCallback,
                                    name: "TLS callback".to_string(),
                                },
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn section(&self, address: u64) -> Option<Section> {
        let file = PeFile64::parse(self.data.as_slice(), self.va_space).unwrap();
        let relative_address = (address - file.relative_address_base()) as u32;
        let Some(section) = file.section_table().section_containing(relative_address) else {
            return None;
        };
        let Some((data_offset, data_length)) = section.pe_range_at(relative_address, self.va_space) else {
            return None;
        };
        Some(Section {
            type_: if section.characteristics.get(LittleEndian)
                & (IMAGE_SCN_CNT_CODE | IMAGE_SCN_MEM_EXECUTE)
                != 0
            {
                SectionType::Assembly
            } else {
                SectionType::Raw
            },
            data_offset: data_offset as usize,
            data_length: data_length as usize,
        })
    }
}
