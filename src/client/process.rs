use std::{ffi::OsString, os::windows::prelude::OsStringExt};

use eframe::egui::{
    Key, KeyboardShortcut, Label, Modifiers, RichText, Sense, TextStyle, Ui, Window,
};
use egui_extras::{Column, TableBuilder};
use windows::Win32::{
    Foundation::{CloseHandle, FALSE, HMODULE, MAX_PATH},
    System::{
        ProcessStatus::{EnumProcesses, EnumProcessModules, GetModuleBaseNameW},
        Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    },
};

pub struct AttachProcessWindow {
    pub(crate) open: bool,
    processes: Vec<(u32, String)>,
}

impl AttachProcessWindow {
    pub fn new() -> Self {
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

    pub fn ui(&mut self, ui: &mut Ui) -> Option<u32> {
        let mut process = None;
        Window::new("Attach Process")
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
                                // pid column
                                row.col(|ui| {
                                    ui.add(
                                        Label::new(RichText::from(format!("{}", pid)).monospace())
                                            .wrap(false),
                                    );
                                });

                                // name column
                                row.col(|ui| {
                                    if ui
                                        .add(
                                            Label::new(RichText::from(name).monospace())
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

        // F5: refresh
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
