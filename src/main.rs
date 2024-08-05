use eframe::egui::{self, Button};
use lifx_core::HSBK;
use std::time::{Duration, Instant};

use mantle::Manager;

const SIZE: [f32; 2] = [320.0, 800.0];
const LIFX_RANGE: std::ops::RangeInclusive<u16> = 0..=u16::MAX;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size(SIZE),
        ..Default::default()
    };

    eframe::run_native(
        "mantle",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::<MantleApp>::default())
        }),
    )
}

struct MantleApp {
    mgr: Manager,
}

impl Default for MantleApp {
    fn default() -> Self {
        let mgr = Manager::new().unwrap();
        Self { mgr }
    }
}

impl eframe::App for MantleApp {
    fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if Instant::now() - self.mgr.last_discovery > Duration::from_secs(10) {
            self.mgr.discover().unwrap();
        }
        self.mgr.refresh();
        egui::CentralPanel::default().show(_ctx, |ui| {
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
                                egui::Slider::new(&mut saturation, LIFX_RANGE).text("Saturation"),
                            );
                            ui.add(
                                egui::Slider::new(&mut brightness, LIFX_RANGE).text("Brightness"),
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
    }
}
