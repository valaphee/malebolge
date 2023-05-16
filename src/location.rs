use eframe::egui::{Label, RichText, TextStyle, Ui};
use egui_dock::egui::Sense;
use egui_extras::{Column, TableBuilder};

use crate::{AppState, AppView};

pub struct LocationView {
    locations: Vec<Location>,
}

impl LocationView {
    pub fn new(locations: Vec<Location>) -> Self {
        Self { locations }
    }
}

impl AppView for LocationView {
    fn title(&self) -> String {
        "Locations".into()
    }

    fn ui(&mut self, state: &mut AppState, ui: &mut Ui) {
        let row_height = ui.text_style_height(&TextStyle::Monospace);
        TableBuilder::new(ui)
            .striped(true)
            .min_scrolled_height(0.0)
            .max_scroll_height(f32::INFINITY)
            .columns(Column::auto().resizable(true), 2)
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
            .body(|body| {
                body.rows(row_height, self.locations.len(), |index, mut row| {
                    let location = &self.locations[index];
                    row.col(|ui| {
                        if ui
                            .add(
                                Label::new(
                                    RichText::from(format!("{:016X}", location.address))
                                        .monospace(),
                                )
                                .wrap(false)
                                .sense(Sense::click()),
                            )
                            .clicked()
                        {
                            state.go_to_address = Some(location.address)
                        }
                    });
                    row.col(|ui| {
                        ui.add(
                            Label::new(
                                RichText::from(match location.type_ {
                                    LocationType::EntryPoint => "Entry point",
                                    LocationType::Export => "Export",
                                    LocationType::TlsCallback => "TLS callback",
                                })
                                .monospace(),
                            )
                            .wrap(false),
                        );
                    });
                    row.col(|ui| {
                        ui.add(Label::new(RichText::from(&location.name).monospace()).wrap(false));
                    });
                });
            });
    }
}

pub struct Location {
    address: u64,
    type_: LocationType,
    name: String,
}

impl Location {
    pub fn new(address: u64, type_: LocationType, name: String) -> Self {
        Self {
            address,
            type_,
            name,
        }
    }
}

pub enum LocationType {
    EntryPoint,
    Export,
    TlsCallback,
}
