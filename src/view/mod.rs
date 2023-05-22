use eframe::egui::{Ui, WidgetText};

use crate::Viewer;

pub mod assembly;
pub mod label;
pub mod raw;

pub trait View {
    fn title(&self) -> String;

    fn ui(&mut self, project: &mut Viewer, ui: &mut Ui);
}
