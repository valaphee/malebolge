use eframe::egui::{Label, RichText, TextStyle, Ui};
use egui_dock::egui::Sense;
use egui_extras::{Column, TableBuilder};

use crate::{
    gui::{AppContext, AppView},
    project,
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
                for (rva, label) in context.project.label_by_rva.iter() {
                    body.row(row_height, |mut row| {
                        // address column
                        row.col(|ui| {
                            if ui
                                .add(
                                    Label::new(RichText::from(format!("{:016X}", rva)).monospace())
                                        .wrap(false)
                                        .sense(Sense::click()),
                                )
                                .context_menu(|ui| {
                                    ui.menu_button("Copy", |ui| {
                                        if ui.button("VA").clicked() {
                                            ui.close_menu();
                                            ui.output_mut(|output| {
                                                output.copied_text =
                                                    format!("{:016X}", context.project.base() + rva)
                                            });
                                        }
                                        if ui.button("RVA").clicked() {
                                            ui.close_menu();
                                            ui.output_mut(|output| {
                                                output.copied_text = format!("{:016X}", rva)
                                            });
                                        }
                                        if ui.button("Name").clicked() {
                                            ui.close_menu();
                                            ui.output_mut(|output| {
                                                output.copied_text = format!("{}", label)
                                            });
                                        }
                                    });
                                })
                                .clicked()
                            {
                                context.open_address_view(*rva);
                            }
                        });

                        // type column
                        row.col(|ui| {
                            ui.add(
                                Label::new(
                                    RichText::from(match label {
                                        project::Label::EntryPoint => "Entry point",
                                        project::Label::Export { .. } => "Export",
                                        project::Label::TlsCallback { .. } => "TLS callback",
                                    })
                                    .monospace(),
                                )
                                .wrap(false),
                            );
                        });

                        // name column
                        row.col(|ui| {
                            ui.add(
                                Label::new(RichText::from(format!("{}", label)).monospace())
                                    .wrap(false),
                            );
                        });
                    });
                }
            });
    }
}
