pub mod screencap;
pub mod waveform;
pub mod widgets;

pub use screencap::*;
pub use waveform::*;
pub use widgets::*;

use crate::app::{ICON, MAIN_WINDOW_SIZE, MIN_WINDOW_SIZE};

use eframe::egui;
use image::GenericImageView;

pub fn setup_eframe_options() -> eframe::NativeOptions {
    let icon = load_icon(ICON);

    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(MAIN_WINDOW_SIZE)
            .with_min_inner_size(MIN_WINDOW_SIZE)
            .with_icon(icon),
        ..Default::default()
    }
}

pub fn load_icon(icon: &[u8]) -> egui::IconData {
    let icon = image::load_from_memory(icon).expect("Failed to load icon");
    egui::IconData {
        rgba: icon.to_rgba8().into_raw(),
        width: icon.width(),
        height: icon.height(),
    }
}
