#![feature(int_roundings)]
#![feature(strict_provenance)]
#![windows_subsystem = "windows"]

use crate::gui::App;

mod asm;
mod gui;
mod project;

pub fn main() -> eframe::Result<()> {
    eframe::run_native(
        "Malebolge",
        eframe::NativeOptions::default(),
        Box::new(|_| Box::new(App::default())),
    )
}
