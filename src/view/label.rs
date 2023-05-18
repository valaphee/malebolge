use eframe::{
    egui,
    egui::{Grid, Layout, RichText, TextStyle, Ui, Window},
};
use egui_dock::egui::{Align, Sense};
use egui_extras::{Column, TableBuilder};

use crate::{AppView, Project};

#[derive(Default)]
pub struct LabelView {
    // runtime
    new_label_window: Option<NewLabelWindow>,
}

impl AppView for LabelView {
    fn title(&self) -> String {
        "Labels".into()
    }

    fn ui(&mut self, project: &mut Project, ui: &mut Ui) {
        // render table
        let row_height = ui.text_style_height(&TextStyle::Monospace);
        TableBuilder::new(ui)
            .striped(true)
            .min_scrolled_height(0.0)
            .max_scroll_height(f32::INFINITY)
            .column(Column::auto())
            .column(Column::auto().resizable(true))
            .column(Column::remainder())
            .header(row_height, |mut row| {
                row.col(|ui| {
                    ui.monospace("Address");
                });
                row.col(|ui| {
                    ui.monospace("Type");
                });
                row.col(|ui| {
                    ui.monospace("Name");
                });
            })
            .body(|mut body| {
                // render rows
                for (label_address, label) in project.labels.iter() {
                    body.row(row_height, |mut row| {
                        // render cols
                        row.col(|ui| {
                            if ui
                                .add(
                                    egui::Label::new(
                                        RichText::from(format!("{:016X}", label_address)).monospace(),
                                    )
                                        .wrap(false)
                                        .sense(Sense::click()),
                                )
                                .context_menu(|ui| {
                                    ui.menu_button("Copy", |ui| {
                                        if ui.button("VA").clicked() {
                                            ui.output_mut(|output| {
                                                output.copied_text = format!("{:016X}", label_address)
                                            });
                                            ui.close_menu();
                                        }
                                        if ui.button("Name").clicked() {
                                            ui.output_mut(|output| {
                                                output.copied_text = label.name.clone()
                                            });
                                            ui.close_menu();
                                        }
                                    });
                                })
                                .clicked()
                            {
                                project.go_to_address = Some(*label_address)
                            }
                        });
                        row.col(|ui| {
                            ui.add(
                                egui::Label::new(
                                    RichText::from(match label.type_ {
                                        LabelType::EntryPoint => "Entry point",
                                        LabelType::Export => "Export",
                                        LabelType::TlsCallback => "TLS callback",
                                        LabelType::Custom => "Custom",
                                    })
                                        .monospace(),
                                )
                                    .wrap(false),
                            );
                        });
                        row.col(|ui| {
                            ui.add(
                                egui::Label::new(RichText::from(&label.name).monospace()).wrap(false),
                            );
                        });
                    });
                }
            });
        // render "add label" window
        if let Some(add_label_window) = &mut self.new_label_window {
            if let Some(label) = add_label_window.ui(ui) {
                self.new_label_window = None;
                project.labels.insert(label.0, label.1);
            } else if !add_label_window.open {
                self.new_label_window = None;
            }
        }
        // render context menu
        ui.interact(
            ui.available_rect_before_wrap(),
            ui.id().with("context_menu"),
            Sense::click(),
        )
        .context_menu(|ui| {
            // open "new label" window
            if ui.button("New Label").clicked() && self.new_label_window.is_none() {
                self.new_label_window = Some(Default::default());
                ui.close_menu();
            }
        });
    }
}

#[derive(Clone)]
pub struct Label {
    type_: LabelType,
    name: String,
}

impl Label {
    pub fn new(type_: LabelType, name: String) -> Self {
        Self {
            type_,
            name,
        }
    }
}

#[derive(Clone)]
pub enum LabelType {
    EntryPoint,
    Export,
    TlsCallback,
    Custom,
}

struct NewLabelWindow {
    open: bool,
    name: String,
    address: String,
}

impl NewLabelWindow {
    fn ui(&mut self, ui: &mut Ui) -> Option<(u64, Label)> {
        let mut label = None;
        Window::new("New Label")
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
                    if ui.button("Add").clicked() {
                        if !self.name.is_empty() {
                            if let Ok(address) = u64::from_str_radix(&self.address, 16) {
                                label =
                                    Some((address, Label::new(LabelType::Custom, self.name.clone())));
                            }
                        }
                    }
                })
            });
        if label.is_some() {
            self.open = false;
        }
        label
    }
}

impl Default for NewLabelWindow {
    fn default() -> Self {
        Self {
            open: true,
            name: Default::default(),
            address: Default::default(),
        }
    }
}
