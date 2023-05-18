#![feature(int_roundings)]
#![windows_subsystem = "windows"]

use byteorder::{ReadBytesExt, LE};
use eframe::{
    egui::{CentralPanel, Context, Frame, Grid, Key, Layout, Ui, WidgetText, Window},
    emath::Align,
};
use egui_dock::{DockArea, Node, Tree};
use object::{
    coff::CoffHeader,
    pe,
    pe::{ImageDosHeader, ImageNtHeaders64, ImageTlsDirectory64},
    read::pe::{ExportTarget, ImageNtHeaders, ImageOptionalHeader},
    LittleEndian, ReadRef,
};

use crate::view::{
    assembly::AssemblyView,
    label::{Label, LabelType, LabelView},
    raw::RawView,
    AppView,
};

mod view;
mod warden;

pub fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Amalgam",
        eframe::NativeOptions {
            icon_data: None,
            ..Default::default()
        },
        Box::new(|_| Box::new(App::new())),
    )
}

#[derive(Default)]
struct App {
    global: Global,
    tree: Tree<Box<dyn AppView>>,
    // runtime
    go_to_address_window: Option<GoToAddressWindow>,
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
            global: Global {
                data,
                ..Default::default()
            },
            ..Default::default()
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
        let data = self.global.data.as_slice();
        let dos_header = ImageDosHeader::parse(data).unwrap();
        let mut nt_header_offset = dos_header.nt_headers_offset().into();
        let (nt_headers, data_directories) =
            ImageNtHeaders64::parse(data, &mut nt_header_offset).unwrap();
        let file_header = nt_headers.file_header();
        let optional_header = nt_headers.optional_header();
        let sections = file_header.sections(data, nt_header_offset).unwrap();
        let mut entries = vec![];
        if optional_header.address_of_entry_point() != 0 {
            entries.push(Label::new(
                optional_header.address_of_entry_point() as u64 + optional_header.image_base(),
                LabelType::EntryPoint,
                "".to_string(),
            ));
        }
        if let Some(export_table) = data_directories.export_table(data, &sections).unwrap() {
            for export in export_table.exports().unwrap() {
                match export.target {
                    ExportTarget::Address(relative_address) => {
                        entries.push(Label::new(
                            relative_address as u64 + optional_header.image_base(),
                            LabelType::Export,
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
                            entries.push(Label::new(
                                callback,
                                LabelType::TlsCallback,
                                "".to_string(),
                            ));
                        }
                    }
                }
            }
        }
        self.open_view(Box::new(LabelView::new(entries)));
    }

    fn open_address_view(&mut self, address: u64) {
        let data = self.global.data.as_slice();
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
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        // toggle fullscreen (F11)
        if ctx.input(|input_state| input_state.key_pressed(Key::F11)) {
            frame.set_fullscreen(!frame.info().window_info.fullscreen)
        }
        // open "go to address" window
        if ctx.input(|input_state| input_state.key_pressed(Key::G))
            && self.go_to_address_window.is_none()
        {
            self.go_to_address_window = Some(Default::default())
        }
        CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                // render dock area
                DockArea::new(&mut self.tree).show_inside(
                    ui,
                    &mut TabViewer {
                        global: &mut self.global,
                    },
                );
                // render "go to address" window
                if let Some(go_to_address_window) = &mut self.go_to_address_window {
                    if let Some(address) = go_to_address_window.ui(ui) {
                        self.go_to_address_window = None;
                        self.global.go_to_address = Some(address);
                    } else if !go_to_address_window.open {
                        self.go_to_address_window = None;
                    }
                }
            });
        // go to address
        if let Some(address) = self.global.go_to_address {
            self.global.go_to_address = None;
            self.open_address_view(address);
        }
    }
}

struct TabViewer<'a> {
    global: &'a mut Global,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = Box<dyn AppView>;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        tab.ui(&mut self.global, ui);
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.title().into()
    }
}

#[derive(Default)]
pub struct Global {
    data: Vec<u8>,
    // event
    go_to_address: Option<u64>,
}

struct GoToAddressWindow {
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
