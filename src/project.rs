use std::{collections::BTreeMap, path::Path};

use byteorder::{ReadBytesExt, LE};
use eframe::egui::{Align, Grid, Layout, Ui, Window};
use object::{
    coff::CoffHeader,
    pe,
    pe::{ImageDosHeader, ImageNtHeaders64, ImageTlsDirectory64},
    read::pe::{ExportTarget, ImageNtHeaders, ImageOptionalHeader},
    LittleEndian, ReadRef,
};

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

    pub fn ui(&mut self, ui: &mut Ui) {
        // render "go to address" window
        if let Some(go_to_address_window) = &mut self.go_to_address_window {
            if let Some(address) = go_to_address_window.ui(ui) {
                self.go_to_address_window = None;
                self.go_to_address = Some(address);
            } else if !go_to_address_window.open {
                self.go_to_address_window = None;
            }
        }
        // render "label" window
        if let Some(label_window) = &mut self.label_window {
            if let Some(label) = label_window.ui(ui) {
                self.label_window = None;
                self.labels.insert(label.0, label.1);
            } else if !label_window.open {
                self.label_window = None;
            }
        }
    }
}

pub struct GoToAddressWindow {
    open: bool,
    address: String,
}

impl GoToAddressWindow {
    fn ui(&mut self, ui: &mut Ui) -> Option<u64> {
        let mut address = None;
        Window::new("Go To Address")
            .open(&mut self.open)
            .resizable(false)
            .collapsible(false)
            .show(ui.ctx(), |ui| {
                Grid::new("").num_columns(2).show(ui, |ui| {
                    ui.label("Address");
                    ui.text_edit_singleline(&mut self.address);
                    ui.end_row();
                });
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    if ui.button("Go").clicked() {
                        if let Ok(address_) = u64::from_str_radix(&self.address, 16) {
                            address = Some(address_);
                        }
                    }
                })
            });
        if address.is_some() {
            self.open = false;
        }
        address
    }
}

impl Default for GoToAddressWindow {
    fn default() -> Self {
        Self {
            open: true,
            address: Default::default(),
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

pub struct LabelWindow {
    open: bool,
    name: String,
    address: String,
}

impl LabelWindow {
    pub fn new(name: String, address: u64) -> Self {
        Self {
            open: true,
            name,
            address: if address == 0 {
                "".to_string()
            } else {
                format!("{:016X}", address)
            },
        }
    }

    fn ui(&mut self, ui: &mut Ui) -> Option<(u64, Label)> {
        let mut label = None;
        let mut close = false;
        Window::new("Label")
            .open(&mut self.open)
            .resizable(false)
            .collapsible(false)
            .show(ui.ctx(), |ui| {
                Grid::new("").num_columns(2).show(ui, |ui| {
                    ui.label("Name");
                    ui.text_edit_singleline(&mut self.name);
                    ui.end_row();
                    ui.label("Address");
                    ui.text_edit_singleline(&mut self.address);
                    ui.end_row();
                });
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    if ui.button("Ok").clicked() {
                        if !self.name.is_empty() {
                            if let Ok(address) = u64::from_str_radix(&self.address, 16) {
                                label = Some((
                                    address,
                                    Label {
                                        type_: LabelType::Custom,
                                        name: self.name.clone(),
                                    },
                                ));
                            }
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                })
            });
        if label.is_some() || close {
            self.open = false;
        }
        label
    }
}

impl Default for LabelWindow {
    fn default() -> Self {
        Self {
            open: true,
            name: Default::default(),
            address: Default::default(),
        }
    }
}
