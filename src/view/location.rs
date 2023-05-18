use eframe::egui::{Grid, Label, RichText, TextStyle, Ui, Window};
use egui_dock::egui::Sense;
use egui_extras::{Column, TableBuilder};

use crate::{AppState, AppView};

pub struct LocationView {
    locations: Vec<Location>,
    // action
    open_add_location: bool,
}

impl LocationView {
    pub fn new(locations: Vec<Location>) -> Self {
        Self { locations, open_add_location: false }
    }
}

impl AppView for LocationView {
    fn title(&self) -> String {
        "Locations".into()
    }

    fn ui(&mut self, state: &mut AppState, ui: &mut Ui) {
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
            .body(|body| {
                // render rows
                body.rows(row_height, self.locations.len(), |index, mut row| {
                    let location = &self.locations[index];
                    // render cols
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
                                    LocationType::Custom => "Custom",
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
        ui.interact(ui.available_rect_before_wrap(), ui.id().with("interact"), Sense::click()).context_menu(|ui| {
            if ui.button("Add Location").clicked() {
                self.open_add_location = true;
                ui.close_menu();
            }
        });
        if self.open_add_location {
            Window::new("Add Location").open(&mut self.open_add_location).show(ui.ctx(), |ui| {
                Grid::new("add_location").show(ui, |ui| {
                    ui.label("Name");
                    let mut name = String::new();
                    ui.text_edit_singleline(&mut name);
                    ui.end_row();
                    ui.label("Address");
                    let mut address = String::new();
                    ui.text_edit_singleline(&mut address);
                    ui.end_row();
                })
            });
        }
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
    Custom,
}
