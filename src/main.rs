#![feature(int_roundings)]
#![windows_subsystem = "windows"]

use std::{collections::BTreeMap, path::Path};

use byteorder::{ReadBytesExt, LE};
use eframe::{
    egui::{Button, CentralPanel, Context, Frame, Grid, Key, Layout, Ui, Vec2, WidgetText, Window},
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
    label::{Label, LabelType, LabelView, LabelWindow},
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
        Box::new(|_| Box::new(App::default())),
    )
}

#[derive(Default)]
struct App {
    project: Option<Project>,
    tree: Tree<Box<dyn AppView>>,
}

impl App {
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

    fn open_label_view(&mut self) {
        self.open_view(Box::new(LabelView::default()));
    }

    fn open_address_view(&mut self, address: u64) {
        let data = self.project.as_ref().unwrap().data.as_slice();
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
        if ctx.input(|input| input.key_pressed(Key::F11)) {
            frame.set_fullscreen(!frame.info().window_info.fullscreen)
        }
        if let Some(project) = &mut self.project {
            // open "go to address" window
            if ctx.input(|input| input.key_pressed(Key::G))
                && project.go_to_address_window.is_none()
            {
                project.go_to_address_window = Some(Default::default())
            }
            // render main
            CentralPanel::default()
                .frame(Frame::none())
                .show(ctx, |ui| {
                    // render dock area
                    DockArea::new(&mut self.tree).show_inside(ui, &mut TabViewer { project });
                    // render "go to address" window
                    if let Some(go_to_address_window) = &mut project.go_to_address_window {
                        if let Some(address) = go_to_address_window.ui(ui) {
                            project.go_to_address_window = None;
                            project.go_to_address = Some(address);
                        } else if !go_to_address_window.open {
                            project.go_to_address_window = None;
                        }
                    }
                    // render "label" window
                    if let Some(label_window) = &mut project.label_window {
                        if let Some(label) = label_window.ui(ui) {
                            project.label_window = None;
                            project.labels.insert(label.0, label.1);
                        } else if !label_window.open {
                            project.label_window = None;
                        }
                    }
                });
            // go to address
            if let Some(address) = project.go_to_address {
                project.go_to_address = None;
                self.open_address_view(address);
            }
            // close project if tree is empty
            if self.tree.is_empty() {
                self.project = None;
            }
        } else {
            CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    // open project
                    if ui
                        .add(Button::new("Open File").min_size(Vec2::new(100.0, 25.0)))
                        .clicked()
                    {
                        let Some(path) = rfd::FileDialog::new().pick_file() else {
                            todo!()
                        };
                        self.project = Some(Project::new(path));
                        self.open_label_view();
                    }
                    if ui
                        .add(Button::new("Attach Process").min_size(Vec2::new(100.0, 25.0)))
                        .clicked()
                    {}
                });
            });
        }
    }
}

struct TabViewer<'a> {
    project: &'a mut Project,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = Box<dyn AppView>;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        tab.ui(self.project, ui);
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.title().into()
    }
}

pub struct Project {
    labels: BTreeMap<u64, Label>,
    data: Vec<u8>,
    // runtime
    go_to_address: Option<u64>,
    go_to_address_window: Option<GoToAddressWindow>,
    label_window: Option<LabelWindow>,
}

impl Project {
    fn new(path: impl AsRef<Path>) -> Self {
        let Ok(data) = std::fs::read(path) else {
            todo!()
        };
        Self {
            labels: Self::parse_labels(&data),
            data,
            go_to_address: None,
            go_to_address_window: None,
            label_window: None,
        }
    }

    fn parse_labels(data: &[u8]) -> BTreeMap<u64, Label> {
        let dos_header = ImageDosHeader::parse(data).unwrap();
        let mut nt_header_offset = dos_header.nt_headers_offset().into();
        let (nt_headers, data_directories) =
            ImageNtHeaders64::parse(data, &mut nt_header_offset).unwrap();
        let file_header = nt_headers.file_header();
        let optional_header = nt_headers.optional_header();
        let sections = file_header.sections(data, nt_header_offset).unwrap();
        let mut labels = BTreeMap::new();
        if optional_header.address_of_entry_point() != 0 {
            labels.insert(
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
                        labels.insert(
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
                            labels.insert(
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
        labels
    }
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
