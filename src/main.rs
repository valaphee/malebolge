#![feature(int_roundings)]
#![feature(strict_provenance)]
#![windows_subsystem = "windows"]

use crate::gui::App;

mod gui;
mod project;

pub fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Amalgam",
        eframe::NativeOptions::default(),
        Box::new(|_| Box::new(App::default())),
    )
}
