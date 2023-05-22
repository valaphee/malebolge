use eframe::{
    egui,
    egui::{RichText, TextStyle, Ui},
};
use egui_dock::egui::Sense;
use egui_extras::{Column, TableBuilder};

use crate::{LabelType, view::View, Viewer};

#[derive(Default)]
pub struct LabelView;

impl View for LabelView {
    fn title(&self) -> String {
        "Labels".into()
    }

    fn ui(&mut self, project: &mut Viewer, ui: &mut Ui) {
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
                for (label_address, label) in project.labels.iter() {
                    body.row(row_height, |mut row| {
                        // address
                        row.col(|ui| {
                            if ui
                                .add(
                                    egui::Label::new(
                                        RichText::from(format!("{:016X}", label_address))
                                            .monospace(),
                                    )
                                    .wrap(false)
                                    .sense(Sense::click()),
                                )
                                .context_menu(|ui| {
                                    ui.menu_button("Copy", |ui| {
                                        if ui.button("VA").clicked() {
                                            ui.output_mut(|output| {
                                                output.copied_text =
                                                    format!("{:016X}", label_address)
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
                                    if ui.button("Remove").clicked() {
                                        remove_label = Some(*label_address);
                                        ui.close_menu();
                                    }
                                })
                                .clicked()
                            {
                                project.go_to_address = Some(*label_address)
                            }
                        });
                        // type
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
                        // name
                        row.col(|ui| {
                            ui.add(
                                egui::Label::new(RichText::from(&label.name).monospace())
                                    .wrap(false),
                            );
                        });
                    });
                }
                // remove label
                if let Some(address) = remove_label {
                    project.labels.remove(&address);
                }
            });
        ui.interact(
            ui.available_rect_before_wrap(),
            ui.id().with("context_menu"),
            Sense::click(),
        )
        .context_menu(|ui| {
            // open "label" window
            if ui.button("Add Label").clicked() && project.label_window.is_none() {
                project.label_window = Some(Default::default());
                ui.close_menu();
            }
        });
    }
}
