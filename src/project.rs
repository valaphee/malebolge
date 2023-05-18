use std::{collections::BTreeMap, path::Path};

use byteorder::{ReadBytesExt, LE};
use object::{
    coff::CoffHeader,
    pe,
    pe::{ImageDosHeader, ImageNtHeaders64, ImageTlsDirectory64},
    read::pe::{ExportTarget, ImageNtHeaders, ImageOptionalHeader},
    LittleEndian, ReadRef,
};

use crate::{GoToAddressWindow, LabelWindow};

pub struct Project {
    pub data: Vec<u8>,
    pub labels: BTreeMap<u64, Label>,
    // runtime
    pub go_to_address: Option<u64>,
    pub go_to_address_window: Option<GoToAddressWindow>,
    pub label_window: Option<LabelWindow>,
}

impl Project {
    pub fn new(path: impl AsRef<Path>) -> Self {
        let Ok(data) = std::fs::read(path) else {
            todo!()
        };
        let mut self_ = Self {
            data,
            labels: Default::default(),
            go_to_address: None,
            go_to_address_window: None,
            label_window: None,
        };
        self_.refresh();
        self_
    }

    pub fn refresh(&mut self) {
        let data = self.data.as_slice();
        let dos_header = ImageDosHeader::parse(data).unwrap();
        let mut nt_header_offset = dos_header.nt_headers_offset().into();
        let (nt_headers, data_directories) =
            ImageNtHeaders64::parse(data, &mut nt_header_offset).unwrap();
        let file_header = nt_headers.file_header();
        let optional_header = nt_headers.optional_header();
        let sections = file_header.sections(data, nt_header_offset).unwrap();
        if optional_header.address_of_entry_point() != 0 {
            self.labels.insert(
                optional_header.address_of_entry_point() as u64 + optional_header.image_base(),
                Label {
                    type_: LabelType::EntryPoint,
                    name: "Entry point".to_string(),
                },
            );
        }
        if let Some(export_table) = data_directories.export_table(data, &sections).unwrap() {
            for export in export_table.exports().unwrap() {
                match export.target {
                    ExportTarget::Address(relative_address) => {
                        self.labels.insert(
                            relative_address as u64 + optional_header.image_base(),
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
        if let Some(directory) = data_directories.get(pe::IMAGE_DIRECTORY_ENTRY_TLS) {
            if let Ok(directory_data) = directory.data(data, &sections) {
                if let Ok(tls_directory) = directory_data.read_at::<ImageTlsDirectory64>(0) {
                    if let Some(mut callback_data) = sections.pe_data_at(
                        data,
                        (tls_directory.address_of_call_backs.get(LittleEndian)
                            - optional_header.image_base()) as u32,
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
}

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
