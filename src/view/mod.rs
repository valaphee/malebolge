use eframe::egui::Ui;

use crate::Project;

pub mod assembly;
pub mod label;
pub mod raw;

pub trait AppView {
    fn title(&self) -> String;

    fn ui(&mut self, project: &mut Project, ui: &mut Ui);
}
