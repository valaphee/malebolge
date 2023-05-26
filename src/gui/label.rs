use eframe::egui::{Label, RichText, TextStyle, Ui};
use egui_dock::egui::Sense;
use egui_extras::{Column, TableBuilder};

use crate::{
    gui::{assembly::AssemblyView, raw::RawView, AppContext, AppView},
    project,
    project::DataViewType,
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
                    let va = context.project.base() + rva;
                    body.row(row_height, |mut row| {
                        // address column
                        row.col(|ui| {
                            if ui
                                .add(
                                    Label::new(RichText::from(format!("{:016X}", va)).monospace())
                                        .wrap(false)
                                        .sense(Sense::click()),
                                )
                                .context_menu(|ui| {
                                    ui.menu_button("Copy", |ui| {
                                        if ui.button("VA").clicked() {
                                            ui.close_menu();
                                            ui.output_mut(|output| {
                                                output.copied_text = format!("{:016X}", va)
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
                                if let Some(data_view) = context.project.data_view(*rva) {
                                    match data_view.type_ {
                                        DataViewType::Raw => {
                                            context
                                                .open_view
                                                .push(Box::new(RawView::new(*rva, data_view)));
                                        }
                                        DataViewType::Assembly => context
                                            .open_view
                                            .push(Box::new(AssemblyView::new(64, *rva, data_view))),
                                    }
                                }
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
