use eframe::egui;
use std::time::{Duration, Instant};

use mantle::Manager;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
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
        if Instant::now() - self.mgr.last_discovery > Duration::from_secs(300) {
            self.mgr.discover().unwrap();
        }
        self.mgr.refresh();
        egui::CentralPanel::default().show(_ctx, |ui| {
            ui.heading("Bulbs");
            let bulbs = self.mgr.bulbs.lock();
            if let Ok(bulbs) = bulbs {
                let bulbs = bulbs.values();
                for bulb in bulbs {
                    ui.label(format!("{:?}", bulb));
                }
            }
        });
    }
}
