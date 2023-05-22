#![feature(int_roundings)]
#![feature(strict_provenance)]
#![windows_subsystem = "windows"]

extern crate core;

use std::{ffi::OsString, os::windows::prelude::*};
use std::collections::BTreeMap;
use std::path::Path;
use byteorder::{LE, ReadBytesExt};

use eframe::{
    egui,
    egui::{
        Align, Button, CentralPanel, Context, Frame, Grid, Key, KeyboardShortcut, Layout,
        Modifiers, RichText, Sense, TextStyle, Ui, Vec2, Window,
    },
};
use eframe::egui::WidgetText;
use egui_dock::{DockArea, Node, Tree};
use egui_extras::{Column, TableBuilder};
use object::{LittleEndian, Object, ReadRef};
use object::pe::{IMAGE_DIRECTORY_ENTRY_TLS, IMAGE_SCN_CNT_CODE, IMAGE_SCN_MEM_EXECUTE, ImageTlsDirectory64};
use object::read::pe::{ExportTarget, ImageNtHeaders, ImageOptionalHeader, PeFile64};
use thiserror::Error;
use windows::Win32::{
    Foundation::{CloseHandle, FALSE, HMODULE, MAX_PATH},
    System::{
        ProcessStatus::{EnumProcessModules, EnumProcesses, GetModuleBaseNameW},
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    },
};
use windows::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows::Win32::System::ProcessStatus::{GetModuleInformation, MODULEINFO};

use crate::{
    view::{assembly::AssemblyView, label::LabelView, raw::RawView, View},
};

mod view;

pub fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Amalgam",
        eframe::NativeOptions::default(),
        Box::new(|_| Box::new(App::default())),
    )
}

#[derive(Default)]
struct App {
    viewer: Option<Viewer>,
    // runtime
    views: Tree<Box<dyn View>>,
    attach_window: Option<AttachWindow>,
}

impl App {
    fn open_view(&mut self, view: Box<dyn View>) {
        let title = view.title();
        if let Some((node_index, tab_index)) =
            self.views
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
            self.views.set_focused_node(node_index.into());
            self.views
                .set_active_tab(node_index.into(), tab_index.into());
        } else {
            self.views.push_to_first_leaf(view)
        }
    }

    fn open_label_view(&mut self) {
        self.open_view(Box::new(LabelView::default()));
    }

    fn open_address_view(&mut self, address: u64) {
        let Some(section) = self.viewer.as_ref().unwrap().section(address) else {
            return;
        };
        match section.type_ {
            SectionType::Raw => {
                self.open_view(Box::new(RawView::new(
                    address,
                    section.data_offset,
                    section.data_length,
                )));
            }
            SectionType::Assembly => {
                self.open_view(Box::new(AssemblyView::new(
                    address,
                    section.data_offset,
                    section.data_length,
                )));
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        if let Some(project) = &mut self.viewer {
            CentralPanel::default()
                .frame(Frame::none())
                .show(ctx, |ui| {
                    DockArea::new(&mut self.views).show_inside(ui, project);
                    if let Some(go_to_address_window) = &mut project.go_to_address_window {
                        if let Some(address) = go_to_address_window.ui(ui) {
                            project.go_to_address_window = None;
                            project.go_to_address = Some(address);
                        } else if !go_to_address_window.open {
                            project.go_to_address_window = None;
                        }
                    }
                    if let Some(label_window) = &mut project.label_window {
                        if let Some(label) = label_window.ui(ui) {
                            project.label_window = None;
                            project.labels.insert(label.0, label.1);
                        } else if !label_window.open {
                            project.label_window = None;
                        }
                    }
                    if ui.input_mut(|input| {
                        input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::G))
                    }) && project.go_to_address_window.is_none()
                    {
                        project.go_to_address_window = Some(Default::default())
                    }
                });
            if let Some(address) = project.go_to_address {
                project.go_to_address = None;
                self.open_address_view(address);
            }
            // close project if tree is empty
            if self.views.is_empty() {
                self.viewer = None;
            }
        } else {
            CentralPanel::default().show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    if ui
                        .add(Button::new("Open File").min_size(Vec2::new(100.0, 25.0)))
                        .clicked()
                    {
                        let Some(path) = rfd::FileDialog::new().pick_file() else {
                            todo!()
                        };
                        self.viewer = Some(Viewer::from_path(path).unwrap());
                        self.open_label_view();
                    }
                    if ui
                        .add(Button::new("Attach Process").min_size(Vec2::new(100.0, 25.0)))
                        .clicked()
                    {
                        self.attach_window = Some(AttachWindow::new());
                    }
                    if ui
                        .add(Button::new("Exit").min_size(Vec2::new(100.0, 25.0)))
                        .clicked()
                    {
                        frame.close()
                    }
                });
                if let Some(attach_window) = &mut self.attach_window {
                    if let Some(pid) = attach_window.ui(ui) {
                        self.viewer = Some(Viewer::from_pid(pid, ctx.clone()).unwrap());
                        self.open_label_view();
                    } else if !attach_window.open {
                        self.attach_window = None;
                    }
                }
            });
        }
        if ctx.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::F11))
        }) {
            frame.set_fullscreen(!frame.info().window_info.fullscreen)
        }
    }
}

pub struct Viewer {
    pub va_space: bool,
    pub data: Vec<u8>,
    pub labels: BTreeMap<u64, Label>,
    // runtime
    pub go_to_address: Option<u64>,
    pub go_to_address_window: Option<GoToAddressWindow>,
    pub label_window: Option<LabelWindow>,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error")]
    Io(#[from] std::io::Error),
    #[error("Windows error")]
    Windows(#[from] windows::core::Error),
}

impl Viewer {
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let data = std::fs::read(path)?;
        let mut project = Self {
            va_space: false,
            data,
            labels: Default::default(),
            go_to_address: None,
            go_to_address_window: None,
            label_window: None,
        };
        project.refresh();
        Ok(project)
    }

    pub fn from_pid(pid: u32, ctx: egui::Context) -> Result<Self> {
        let data = unsafe {
            let process = OpenProcess(
                PROCESS_VM_READ | PROCESS_QUERY_INFORMATION,
                FALSE,
                pid,
            )?;
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
            go_to_address: None,
            go_to_address_window: None,
            label_window: None,
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

impl egui_dock::TabViewer for Viewer {
    type Tab = Box<dyn View>;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        tab.ui(self, ui);
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.title().into()
    }
}

struct AttachWindow {
    open: bool,
    processes: Vec<(u32, String)>,
}

impl AttachWindow {
    fn new() -> Self {
        let mut self_ = Self {
            open: true,
            processes: Default::default(),
        };
        self_.refresh();
        self_
    }

    fn refresh(&mut self) {
        self.processes.clear();
        unsafe {
            let mut pids = [0; 4096];
            let mut pids_length = 0;
            EnumProcesses(
                pids.as_mut_ptr(),
                std::mem::size_of_val(&pids) as u32,
                &mut pids_length,
            )
            .ok()
            .unwrap();
            for &pid in &pids[..pids_length as usize / std::mem::size_of::<u32>()] {
                let Ok(process) = OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, FALSE, pid) else {
                    continue;
                };
                let mut module = HMODULE::default();
                if EnumProcessModules(
                    process,
                    &mut module,
                    std::mem::size_of_val(&module) as u32,
                    &mut 0,
                )
                .ok()
                .is_err()
                {
                    continue;
                }
                let mut module_base_name = [0; MAX_PATH as usize];
                GetModuleBaseNameW(process, module, &mut module_base_name);
                self.processes.push((
                    pid,
                    OsString::from_wide(module_base_name.split(|&elem| elem == 0).next().unwrap())
                        .into_string()
                        .ok()
                        .unwrap(),
                ));
                CloseHandle(process);
            }
        }
        self.processes.sort_by_key(|(_, name)| name.to_lowercase());
    }

    fn ui(&mut self, ui: &mut Ui) -> Option<u32> {
        let mut process = None;
        Window::new("Attach")
            .open(&mut self.open)
            .collapsible(false)
            .show(ui.ctx(), |ui| {
                let row_height = ui.text_style_height(&TextStyle::Monospace);
                TableBuilder::new(ui)
                    .min_scrolled_height(0.0)
                    .max_scroll_height(f32::INFINITY)
                    .column(Column::auto())
                    .column(Column::remainder())
                    .header(row_height, |mut row| {
                        row.col(|ui| {
                            ui.monospace("PID");
                        });
                        row.col(|ui| {
                            ui.monospace("Name");
                        });
                    })
                    .body(|mut body| {
                        for (pid, name) in &self.processes {
                            body.row(row_height, |mut row| {
                                // pid
                                row.col(|ui| {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::from(format!("{}", pid)).monospace(),
                                        )
                                        .wrap(false),
                                    );
                                });
                                // name
                                row.col(|ui| {
                                    if ui
                                        .add(
                                            egui::Label::new(RichText::from(name).monospace())
                                                .wrap(false)
                                                .sense(Sense::click()),
                                        )
                                        .clicked()
                                    {
                                        process = Some(*pid);
                                    }
                                });
                            });
                        }
                    });
            });
        if ui.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::F5))
        }) {
            self.refresh();
        }
        if process.is_some() {
            self.open = false;
        }
        process
    }
}

pub struct GoToAddressWindow {
    open: bool,
    address: String,
}

impl GoToAddressWindow {
    fn ui(&mut self, ui: &mut Ui) -> Option<u64> {
        let mut address = None;
        let mut close = false;
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
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                })
            });
        if address.is_some() || close {
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
