use eframe::egui::{self, Modifiers, Slider, Ui, Vec2};
use lifx_core::HSBK;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::MutexGuard,
    time::{Duration, Instant},
};

use mantle::{
    bulb_info::DeviceInfo, capitalize_first_letter, color::default_hsbk, display_color_circle,
    toggle_button, BulbInfo, Manager,
};

const MAIN_WINDOW_SIZE: [f32; 2] = [320.0, 800.0];
const ABOUT_WINDOW_SIZE: [f32; 2] = [320.0, 480.0];
const MIN_WINDOW_SIZE: [f32; 2] = [300.0, 220.0];
const LIFX_RANGE: std::ops::RangeInclusive<u16> = 0..=u16::MAX;
const KELVIN_RANGE: std::ops::RangeInclusive<u16> = 1500..=9000;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(MAIN_WINDOW_SIZE)
            .with_min_inner_size(MIN_WINDOW_SIZE),
        ..Default::default()
    };

    eframe::run_native(
        "Mantle",
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
    show_about: bool,
}

impl Default for MantleApp {
    fn default() -> Self {
        let mgr = Manager::new().unwrap();
        Self {
            mgr,
            show_about: false,
        }
    }
}

impl MantleApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }

    fn sort_bulbs<'a>(&self, mut bulbs: Vec<&'a BulbInfo>) -> Vec<&'a BulbInfo> {
        bulbs.sort_by(|a, b| {
            let group_a = a
                .group
                .data
                .as_ref()
                .and_then(|g| g.label.cstr().to_str().ok())
                .unwrap_or_default();
            let group_b = b
                .group
                .data
                .as_ref()
                .and_then(|g| g.label.cstr().to_str().ok())
                .unwrap_or_default();
            let name_a = a
                .name
                .data
                .as_ref()
                .and_then(|s| s.to_str().ok())
                .unwrap_or_default();
            let name_b = b
                .name
                .data
                .as_ref()
                .and_then(|s| s.to_str().ok())
                .unwrap_or_default();

            group_a.cmp(group_b).then(name_a.cmp(name_b))
        });
        bulbs
    }

    fn display_device(
        &self,
        ui: &mut Ui,
        device: &DeviceInfo,
        bulbs: &MutexGuard<HashMap<u64, BulbInfo>>,
    ) {
        let color = match device {
            DeviceInfo::Bulb(bulb) => {
                if let Some(s) = bulb.name.data.as_ref().and_then(|s| s.to_str().ok()) {
                    ui.label(s);
                }
                bulb.get_color().cloned()
            }
            DeviceInfo::Group(group) => {
                if let Ok(s) = group.label.cstr().to_str() {
                    ui.label(s);
                }
                Some(self.mgr.avg_group_color(group, bulbs))
            }
        };

        ui.horizontal(|ui| {
            display_color_circle(
                ui,
                device,
                color.unwrap_or(default_hsbk()),
                Vec2::new(1.0, 1.0),
                8.0,
                bulbs,
            );

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Power");
                    toggle_button(ui, &self.mgr, device, Vec2::new(1.0, 1.0), bulbs);
                });
                if let Some(before_color) = color {
                    let after_color =
                        self.display_color_controls(ui, device, color.unwrap_or(default_hsbk()));
                    if before_color != after_color {
                        match device {
                            DeviceInfo::Bulb(bulb) => {
                                if let Err(e) = self.mgr.set_color(bulb, after_color) {
                                    println!("Error setting color: {}", e);
                                }
                            }
                            DeviceInfo::Group(group) => {
                                if let Err(e) = self.mgr.set_group_color(group, after_color, bulbs)
                                {
                                    println!("Error setting group color: {}", e);
                                }
                            }
                        }
                    }
                }
            });
        });
        ui.separator();
    }

    fn display_color_controls(&self, ui: &mut Ui, device: &DeviceInfo, color: HSBK) -> HSBK {
        let HSBK {
            mut hue,
            mut saturation,
            mut brightness,
            mut kelvin,
        } = color;
        ui.vertical(|ui| {
            ui.add(Slider::new(&mut hue, LIFX_RANGE).text("Hue"));
            ui.add(Slider::new(&mut saturation, LIFX_RANGE).text("Saturation"));
            ui.add(Slider::new(&mut brightness, LIFX_RANGE).text("Brightness"));
            match device {
                DeviceInfo::Bulb(bulb) => {
                    if let Some(range) = bulb.features.temperature_range.as_ref() {
                        if range.min != range.max {
                            ui.add(Slider::new(&mut kelvin, range.to_range_u16()).text("Kelvin"));
                        } else {
                            ui.label(format!("Kelvin: {:?}", range.min));
                        }
                    }
                }
                DeviceInfo::Group(_) => {
                    ui.add(Slider::new(&mut kelvin, KELVIN_RANGE).text("Kelvin"));
                }
            }
        });
        HSBK {
            hue,
            saturation,
            brightness,
            kelvin,
        }
    }

    fn file_menu_button(&self, ui: &mut Ui) {
        let close_shortcut = egui::KeyboardShortcut::new(Modifiers::CTRL, egui::Key::Q);
        let refresh_shortcut = egui::KeyboardShortcut::new(Modifiers::NONE, egui::Key::F5);
        if ui.input_mut(|i| i.consume_shortcut(&close_shortcut)) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if ui.input_mut(|i| i.consume_shortcut(&refresh_shortcut)) {
            self.mgr.refresh();
        }

        ui.menu_button("File", |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            if ui
                .add(
                    egui::Button::new("Refresh")
                        .shortcut_text(ui.ctx().format_shortcut(&refresh_shortcut)),
                )
                .clicked()
            {
                self.mgr.refresh();
                ui.close_menu();
            }
            if ui
                .add(
                    egui::Button::new("Quit")
                        .shortcut_text(ui.ctx().format_shortcut(&close_shortcut)),
                )
                .clicked()
            {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                ui.close_menu();
            }
        });
    }

    fn help_menu_button(&mut self, ui: &mut Ui) {
        ui.menu_button("Help", |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            if ui.add(egui::Button::new("About")).clicked() {
                self.show_about = true;
                ui.close_menu();
            }
        });
    }
}

impl eframe::App for MantleApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if Instant::now() - self.mgr.last_discovery > Duration::from_secs(3) {
            self.mgr.discover().unwrap();
        }
        self.mgr.refresh();
        egui::TopBottomPanel::top("menu_bar").show(_ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                self.file_menu_button(ui);
                self.help_menu_button(ui);
            });
        });
        egui::CentralPanel::default().show(_ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let bulbs = self.mgr.bulbs.lock();
                let mut seen_groups = HashSet::<String>::new();
                ui.vertical(|ui| {
                    if let Ok(bulbs) = bulbs {
                        let sorted_bulbs = self.sort_bulbs(bulbs.values().collect());
                        for bulb in sorted_bulbs {
                            if let Some(group) = bulb.group.data.as_ref() {
                                let group_name = group.label.cstr().to_str().unwrap_or_default();
                                if !seen_groups.contains(group_name) {
                                    seen_groups.insert(group_name.to_owned());
                                    self.display_device(
                                        ui,
                                        &DeviceInfo::Group(group.clone()),
                                        &bulbs,
                                    );
                                }
                            }
                            self.display_device(ui, &DeviceInfo::Bulb(bulb), &bulbs);
                        }
                    }
                });
            });
        });
        if self.show_about {
            egui::Window::new("About")
                .default_width(ABOUT_WINDOW_SIZE[0])
                .default_height(ABOUT_WINDOW_SIZE[1])
                .open(&mut self.show_about)
                .resizable([true, false])
                .show(_ctx, |ui| {
                    ui.heading(capitalize_first_letter(env!("CARGO_PKG_NAME")));
                    ui.add_space(8.0);
                    ui.label(env!("CARGO_PKG_DESCRIPTION"));
                    ui.label(format!("Version: {}", env!("CARGO_PKG_VERSION")));
                    ui.label(format!("Author: {}", env!("CARGO_PKG_AUTHORS")));
                    ui.hyperlink_to("Github", env!("CARGO_PKG_REPOSITORY"));
                });
        }
    }
}
