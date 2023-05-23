use eframe::egui::{Align, Grid, Label, Layout, RichText, TextStyle, Ui, Window};
use egui_dock::egui::Sense;
use egui_extras::{Column, TableBuilder};

use crate::{
    gui::{AppContext, AppView},
    project::LabelType,
};

#[derive(Default)]
pub struct LabelView;

impl AppView for LabelView {
    fn title(&self) -> String {
        "Labels".into()
    }

    fn ui(&mut self, context: &mut AppContext, ui: &mut Ui) {
        let row_height = ui.text_style_height(&TextStyle::Monospace);
        TableBuilder::new(ui)
            .striped(true)
            .min_scrolled_height(0.0)
            .max_scroll_height(f32::INFINITY)
            .columns(Column::auto(), 2)
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
                let mut remove_label = None;
                for (label_address, label) in context.project.labels.iter() {
                    body.row(row_height, |mut row| {
                        // address column
                        row.col(|ui| {
                            if ui
                                .add(
                                    Label::new(
                                        RichText::from(format!("{:016X}", label_address))
                                            .monospace(),
                                    )
                                    .wrap(false)
                                    .sense(Sense::click()),
                                )
                                .context_menu(|ui| {
                                    ui.menu_button("Copy", |ui| {
                                        if ui.button("VA").clicked() {
                                            ui.close_menu();
                                            ui.output_mut(|output| {
                                                output.copied_text =
                                                    format!("{:016X}", label_address)
                                            });
                                        }
                                        if ui.button("Name").clicked() {
                                            ui.close_menu();
                                            ui.output_mut(|output| {
                                                output.copied_text = label.name.clone()
                                            });
                                        }
                                    });
                                    if ui.button("Remove Label").clicked() {
                                        ui.close_menu();
                                        remove_label = Some(*label_address);
                                    }
                                })
                                .clicked()
                            {
                                context.go_to_address = Some(*label_address);
                            }
                        });

                        // type column
                        row.col(|ui| {
                            ui.add(
                                Label::new(
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

                        // name column
                        row.col(|ui| {
                            ui.add(Label::new(RichText::from(&label.name).monospace()).wrap(false));
                        });
                    });
                }
                if let Some(label) = remove_label {
                    context.project.labels.remove(&label);
                }
            });

        ui.interact(
            ui.available_rect_before_wrap(),
            ui.id().with(""),
            Sense::click(),
        )
        .context_menu(|ui| {
            if ui.button("New Label").clicked() {
                context.label_window = Some(LabelWindow::default());
                ui.close_menu();
            }
        });
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
            address: format!("{:016X}", address),
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) -> Option<(u64, crate::project::Label)> {
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
                                    crate::project::Label {
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

    pub fn open(&self) -> bool {
        self.open
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
