#![feature(int_roundings)]

use byteorder::{ReadBytesExt, LE};
use eframe::egui::{Context, Ui, WidgetText};
use egui_dock::{DockArea, Node, Tree};
use object::{
    coff::CoffHeader,
    pe,
    pe::{ImageDosHeader, ImageNtHeaders64, ImageTlsDirectory64},
    read::pe::{ExportTarget, ImageNtHeaders, ImageOptionalHeader},
    LittleEndian, ReadRef,
};

use crate::{
    assembly::AssemblyView,
    location::{Location, LocationType, LocationView},
    raw::RawView,
};

mod assembly;
mod location;
mod raw;

pub fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Amalgam",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Box::new(App::new())),
    )
}

struct App {
    state: AppState,
    tree: Tree<Box<dyn AppView>>,
}

impl App {
    fn new() -> Self {
        let Some(path) = rfd::FileDialog::new().pick_file() else {
            todo!()
        };
        let Ok(data) = std::fs::read(path) else {
            todo!()
        };

        let mut self_ = Self {
            state: AppState {
                data,
                go_to_address: None,
            },
            tree: Default::default(),
        };
        self_.open_location_view();
        self_
    }

    fn open_view(&mut self, view: Box<dyn AppView>) {
        let title = view.title();
        if let Some((node_index, tab_index)) =
            self.tree
                .iter()
                .enumerate()
                .find_map(|(node_index, node)| match node {
                    Node::Leaf { tabs, .. } => {
                        if let Some((tab_index, _)) = tabs
                            .iter()
                            .enumerate()
                            .find(|(_, tab)| tab.title() == title)
                        {
                            Some((node_index, tab_index))
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
        {
            self.tree.set_focused_node(node_index.into());
            self.tree
                .set_active_tab(node_index.into(), tab_index.into());
        } else {
            self.tree.push_to_first_leaf(view)
        }
    }

    fn open_location_view(&mut self) {
        let data = self.state.data.as_slice();
        let dos_header = ImageDosHeader::parse(data).unwrap();
        let mut nt_header_offset = dos_header.nt_headers_offset().into();
        let (nt_headers, data_directories) =
            ImageNtHeaders64::parse(data, &mut nt_header_offset).unwrap();
        let file_header = nt_headers.file_header();
        let optional_header = nt_headers.optional_header();
        let sections = file_header.sections(data, nt_header_offset).unwrap();
        let mut entries = vec![];
        if optional_header.address_of_entry_point() != 0 {
            entries.push(Location::new(
                optional_header.address_of_entry_point() as u64 + optional_header.image_base(),
                LocationType::EntryPoint,
                "".to_string(),
            ));
        }
        if let Some(export_table) = data_directories.export_table(data, &sections).unwrap() {
            for export in export_table.exports().unwrap() {
                match export.target {
                    ExportTarget::Address(rva) => {
                        entries.push(Location::new(
                            rva as u64 + optional_header.image_base(),
                            LocationType::Export,
                            String::from_utf8_lossy(export.name.unwrap()).to_string(),
                        ));
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
                            entries.push(Location::new(
                                callback,
                                LocationType::TlsCallback,
                                "".to_string(),
                            ));
                        }
                    }
                }
            }
        }
        self.open_view(Box::new(LocationView::new(entries)));
    }

    fn open_address_view(&mut self, address: u64) {
        let data = self.state.data.as_slice();
        let dos_header = ImageDosHeader::parse(data).unwrap();
        let mut nt_header_offset = dos_header.nt_headers_offset().into();
        let (nt_headers, _data_directories) =
            ImageNtHeaders64::parse(data, &mut nt_header_offset).unwrap();
        let file_header = nt_headers.file_header();
        if file_header.machine.get(LittleEndian) != pe::IMAGE_FILE_MACHINE_I386
            && file_header.machine.get(LittleEndian) != pe::IMAGE_FILE_MACHINE_AMD64
        {
            return;
        }
        let optional_header = nt_headers.optional_header();
        let sections = file_header.sections(data, nt_header_offset).unwrap();
        let relative_address = (address - optional_header.image_base()) as u32;
        let Some(section) = sections.section_containing(relative_address) else {
            return;
        };
        let Some((section_data_offset, section_data_length)) = section.pe_file_range_at(relative_address) else {
            return;
        };
        let section_characteristics = section.characteristics.get(LittleEndian);
        if section_characteristics & (pe::IMAGE_SCN_CNT_CODE | pe::IMAGE_SCN_MEM_EXECUTE) != 0 {
            // open assembly view as section is executable
            self.open_view(Box::new(AssemblyView::new(
                if file_header.machine.get(LittleEndian) == pe::IMAGE_FILE_MACHINE_I386 {
                    32
                } else {
                    64
                },
                address,
                section_data_offset as usize,
                section_data_length as usize,
            )));
        } else {
            // open raw view as section contains arbitrary data
            self.open_view(Box::new(RawView::new(
                address,
                section_data_offset as usize,
                section_data_length as usize,
            )));
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        if let Some(go_to_address) = &self.state.go_to_address {
            self.open_address_view(*go_to_address);
            self.state.go_to_address = None;
        }
        DockArea::new(&mut self.tree).show(
            ctx,
            &mut TabViewer {
                state: &mut self.state,
            },
        );
    }
}

struct TabViewer<'a> {
    state: &'a mut AppState,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = Box<dyn AppView>;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        tab.ui(&mut self.state, ui);
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.title().into()
    }
}

trait AppView {
    fn title(&self) -> String;

    fn ui(&mut self, state: &mut AppState, ui: &mut Ui);
}

struct AppState {
    data: Vec<u8>,
    // action
    go_to_address: Option<u64>,
}
