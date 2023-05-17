use eframe::egui::Ui;

use crate::AppState;

pub mod assembly;
pub mod location;
pub mod raw;

pub trait AppView {
    fn title(&self) -> String;

    fn ui(&mut self, state: &mut AppState, ui: &mut Ui);
}
