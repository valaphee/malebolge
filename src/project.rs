use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
    ops::Range,
    path::Path,
};

use byteorder::{ReadBytesExt, LE};
use object::{
    pe::{
        ImageTlsDirectory64, IMAGE_DIRECTORY_ENTRY_TLS, IMAGE_SCN_CNT_CODE, IMAGE_SCN_MEM_EXECUTE,
    },
    read::{
        pe,
        pe::{ExportTarget, ImageNtHeaders, ImageOptionalHeader},
    },
    FileKind, LittleEndian, Object, ReadRef,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("Object read error")]
    Object(#[from] object::read::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Project {
    data: Vec<u8>,
    base: u64,

    pub label_by_rva: BTreeMap<u64, Label>,
}

pub enum Label {
    EntryPoint,
    Export { ordinal: u32, name: String },
    TlsCallback { ordinal: u32 },
}

pub struct DataView {
    pub type_: DataViewType,
    pub range: Range<usize>,
}

pub enum DataViewType {
    Raw,
    Assembly,
}

impl Display for Label {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Label::EntryPoint => f.write_str("Entry point"),
            Label::Export { name, .. } => f.write_str(name),
            Label::TlsCallback { .. } => f.write_str("TLS callback"),
        }
    }
}

impl Project {
    pub fn new(path: impl AsRef<Path>) -> Result<Self> {
        let data = std::fs::read(path)?;
        if !matches!(FileKind::parse(&data[..])?, FileKind::Pe64) {
            todo!()
        }

        let mut self_ = Self {
            data,
            base: Default::default(),
            label_by_rva: Default::default(),
        };
        let file = pe::PeFile64::parse(&self_.data[..], false)?;

        // set base
        self_.base = file.relative_address_base();

        // add entry point label
        if file.nt_headers().optional_header().address_of_entry_point() != 0 {
            self_.label_by_rva.insert(
                file.nt_headers().optional_header().address_of_entry_point() as u64,
                Label::EntryPoint,
            );
        }

        // add export labels
        if let Some(export_table) = file.export_table()? {
            for export in export_table.exports().unwrap() {
                match export.target {
                    ExportTarget::Address(rva) => {
                        self_.label_by_rva.insert(
                            rva as u64,
                            Label::Export {
                                ordinal: export.ordinal,
                                name: String::from_utf8(export.name.unwrap().to_vec()).unwrap(),
                            },
                        );
                    }
                    _ => {}
                }
            }
        }

        // add tls callback labels
        if let Some(directory) = file.data_directory(IMAGE_DIRECTORY_ENTRY_TLS) {
            if let Ok(directory_data) = directory.data(file.data(), &file.section_table()) {
                if let Ok(tls_directory) = directory_data.read_at::<ImageTlsDirectory64>(0) {
                    if let Some(mut callback_data) = file.section_table().pe_data_at(
                        file.data(),
                        (tls_directory.address_of_call_backs.get(LittleEndian)
                            - file.relative_address_base()) as u32,
                    ) {
                        let mut ordinal = 0;
                        loop {
                            let va = callback_data.read_u64::<LE>().unwrap();
                            if va == 0 {
                                break;
                            }
                            self_.label_by_rva.insert(
                                va - file.relative_address_base(),
                                Label::TlsCallback { ordinal },
                            );
                            ordinal += 1;
                        }
                    }
                }
            }
        }

        Ok(self_)
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn data_view(&self, rva: u64) -> Option<DataView> {
        let file = pe::PeFile64::parse(self.data(), false).unwrap();
        let Some(section) = file.section_table().section_containing(rva as u32) else {
            return None;
        };
        let Some((data_offset, data_length)) = section.pe_range_at(rva as u32, false) else {
            return None;
        };
        Some(DataView {
            type_: if section.characteristics.get(LittleEndian)
                & (IMAGE_SCN_CNT_CODE | IMAGE_SCN_MEM_EXECUTE)
                != 0
            {
                DataViewType::Assembly
            } else {
                DataViewType::Raw
            },
            range: data_offset as usize..data_offset as usize + data_length as usize,
        })
    }

    pub fn base(&self) -> u64 {
        self.base
    }
}
