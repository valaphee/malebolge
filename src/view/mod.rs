use eframe::egui::Ui;

use crate::Global;

pub mod assembly;
pub mod label;
pub mod raw;

pub trait AppView {
    fn title(&self) -> String;

    fn ui(&mut self, global: &mut Global, ui: &mut Ui);
}
