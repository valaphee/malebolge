use eframe::egui::{Ui, WidgetText};

use crate::project::Project;

pub mod assembly;
pub mod label;
pub mod raw;

pub trait Tab {
    fn title(&self) -> String;

    fn ui(&mut self, project: &mut Project, ui: &mut Ui);
}

pub struct TabViewer<'a> {
    pub project: &'a mut Project,
}

impl egui_dock::TabViewer for TabViewer<'_> {
    type Tab = Box<dyn Tab>;

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        tab.ui(self.project, ui);
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        tab.title().into()
    }
}
