#![feature(int_roundings)]
#![feature(strict_provenance)]
#![windows_subsystem = "windows"]

extern crate core;

use std::{ffi::OsString, os::windows::prelude::*};

use eframe::{
    egui,
    egui::{
        Align, Button, CentralPanel, Context, Frame, Grid, Key, KeyboardShortcut, Layout,
        Modifiers, RichText, Sense, TextStyle, Ui, Vec2, Window,
    },
};
use egui_dock::{DockArea, Node, Tree};
use egui_extras::{Column, TableBuilder};
use windows::Win32::{
    Foundation::{CloseHandle, FALSE, HMODULE, MAX_PATH},
    System::{
        ProcessStatus::{EnumProcessModules, EnumProcesses, GetModuleBaseNameW},
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    },
};

use crate::{
    project::{Label, LabelType, Project, SectionType},
    tab::{assembly::AssemblyTab, label::LabelTab, raw::RawView, Tab, TabViewer},
};

mod project;
mod tab;
mod util;
mod debug;

pub fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Amalgam",
        eframe::NativeOptions::default(),
        Box::new(|_| Box::new(App::default())),
    )
}

#[derive(Default)]
struct App {
    project: Option<Project>,
    // runtime
    tree: Tree<Box<dyn Tab>>,
    attach_window: Option<AttachWindow>,
}

impl App {
    fn open_view(&mut self, view: Box<dyn Tab>) {
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
        self.open_view(Box::new(LabelTab::default()));
    }

    fn open_address_view(&mut self, address: u64) {
        let Some(section) = self.project.as_ref().unwrap().section(address) else {
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
                self.open_view(Box::new(AssemblyTab::new(
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
        if let Some(project) = &mut self.project {
            CentralPanel::default()
                .frame(Frame::none())
                .show(ctx, |ui| {
                    DockArea::new(&mut self.tree).show_inside(ui, &mut TabViewer { project });
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
            if self.tree.is_empty() {
                self.project = None;
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
                        self.project = Some(Project::from_path(path).unwrap());
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
                        self.project = Some(Project::from_pid(pid, ctx.clone()).unwrap());
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
