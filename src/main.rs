use eframe::egui::{self, Button};
use lifx_core::HSBK;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use mantle::Manager;

const SIZE: [f32; 2] = [320.0, 800.0];
const MIN_SIZE: [f32; 2] = [300.0, 220.0];
const LIFX_RANGE: std::ops::RangeInclusive<u16> = 0..=u16::MAX;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(SIZE)
            .with_min_inner_size(MIN_SIZE),
        ..Default::default()
    };

    eframe::run_native(
        "mantle",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MantleApp::new(cc)))
        }),
    )
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
struct MantleApp {
    #[serde(skip)]
    mgr: Manager,
}

impl Default for MantleApp {
    fn default() -> Self {
        let mgr = Manager::new().unwrap();
        Self { mgr }
    }
}

impl MantleApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }
}

impl eframe::App for MantleApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if Instant::now() - self.mgr.last_discovery > Duration::from_secs(10) {
            self.mgr.discover().unwrap();
        }
        self.mgr.refresh();
        egui::CentralPanel::default().show(_ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Devices");
                ui.vertical(|ui| {
                    let bulbs = self.mgr.bulbs.lock();
                    if let Ok(bulbs) = bulbs {
                        let bulbs = bulbs.values();
                        for bulb in bulbs {
                            if let Some(s) = bulb.name.data.as_ref().and_then(|s| s.to_str().ok()) {
                                ui.label(s);
                            }

                            if ui.add(Button::new("Toggle")).clicked() {
                                if let Err(e) = self.mgr.toggle(&bulb) {
                                    println!("Error toggling bulb: {}", e);
                                } else {
                                    println!("Toggled bulb {:?}", bulb.name);
                                }
                            }
                            if let Some(color) = bulb.get_color() {
                                let HSBK {
                                    mut hue,
                                    mut saturation,
                                    mut brightness,
                                    mut kelvin,
                                } = color;
                                ui.add(egui::Slider::new(&mut hue, LIFX_RANGE).text("Hue"));
                                ui.add(
                                    egui::Slider::new(&mut saturation, LIFX_RANGE)
                                        .text("Saturation"),
                                );
                                ui.add(
                                    egui::Slider::new(&mut brightness, LIFX_RANGE)
                                        .text("Brightness"),
                                );
                                if let Some(range) = bulb.features.temperature_range.as_ref() {
                                    if range.min != range.max {
                                        ui.add(
                                            egui::Slider::new(&mut kelvin, range.to_range_u16())
                                                .text("Kelvin"),
                                        );
                                    }
                                }
                                match self.mgr.set_color(
                                    &bulb,
                                    HSBK {
                                        hue,
                                        saturation,
                                        brightness,
                                        kelvin,
                                    },
                                ) {
                                    Ok(_) => (),
                                    Err(e) => println!("Error setting brightness: {}", e),
                                }
                            }
                            ui.separator();
                        }
                    }
                });
            });
        });
    }
}
