use std::{
    collections::{HashMap, HashSet},
    sync::{mpsc, Arc, Mutex, MutexGuard},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use crate::{
    capitalize_first_letter,
    color::{default_hsbk, DeltaColor},
    device_info::DeviceInfo,
    display_color_circle,
    listener::input_listener::InputListener,
    screencap::{FollowType, ScreenSubregion},
    settings::Settings,
    shortcut::ShortcutManager,
    toggle_button,
    ui::{handle_eyedropper, handle_screencap, hsbk_sliders},
    BulbInfo, LifxManager, ScreencapManager,
};

use eframe::egui::{self, Modifiers, RichText, Ui, Vec2};
use lifx_core::HSBK;
use serde::{Deserialize, Serialize};

// UI and window size constants
pub const MAIN_WINDOW_SIZE: [f32; 2] = [320.0, 800.0];
pub const ABOUT_WINDOW_SIZE: [f32; 2] = [320.0, 480.0];
pub const MIN_WINDOW_SIZE: [f32; 2] = [300.0, 220.0];

// Icon data
pub const ICON: &[u8; 1751] = include_bytes!("../res/logo32.png");
pub const EYEDROPPER_ICON: &[u8; 238] = include_bytes!("../res/icons/color-picker.png");
pub const MONITOR_ICON: &[u8; 204] = include_bytes!("../res/icons/device-desktop.png");
pub const SUBREGION_ICON: &[u8; 218] = include_bytes!("../res/icons/square.png");

#[derive(Debug, Clone)]
pub struct RunningWaveform {
    pub active: bool,
    pub last_update: Instant,
    pub follow_type: FollowType,
    pub stop_tx: Option<mpsc::Sender<()>>,
}
pub struct ColorChannelEntry {
    pub tx: mpsc::Sender<HSBK>,
    pub rx: mpsc::Receiver<HSBK>,
    pub handle: Option<JoinHandle<()>>,
}
pub type ColorChannel = HashMap<u64, ColorChannelEntry>;

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct MantleApp {
    #[serde(skip)]
    pub mgr: LifxManager,
    #[serde(skip)]
    pub screen_manager: ScreencapManager,
    #[serde(skip)]
    pub input_listener: InputListener,
    #[serde(skip)]
    pub shortcut_manager: ShortcutManager,
    #[serde(skip)]
    pub shortcut_handle: Option<JoinHandle<()>>,
    #[serde(skip)]
    pub listener_handle: Option<JoinHandle<()>>,
    pub show_about: bool,
    pub show_settings: bool,
    pub show_eyedropper: HashMap<u64, bool>,
    pub show_subregion: HashMap<u64, bool>,
    pub subregion_points: HashMap<u64, Arc<Mutex<ScreenSubregion>>>,
    pub settings: Settings,
    #[serde(skip)]
    pub waveform_map: HashMap<u64, RunningWaveform>,
    #[serde(skip)]
    pub waveform_channel: ColorChannel,
}

impl Default for MantleApp {
    fn default() -> Self {
        let input_listener = InputListener::new();
        let listener_handle = Some(input_listener.start());
        let shortcut_manager = ShortcutManager::new(input_listener.clone());
        let lifx_manager = LifxManager::new().expect("Failed to create manager");
        let shortcut_handle = Some(shortcut_manager.start(lifx_manager.clone()));
        Self {
            mgr: lifx_manager,
            screen_manager: ScreencapManager::new().expect("Failed to create screen manager"),
            input_listener,
            shortcut_manager,
            shortcut_handle,
            listener_handle,
            show_about: false,
            show_settings: false,
            show_eyedropper: HashMap::new(),
            show_subregion: HashMap::new(),
            subregion_points: HashMap::new(),
            settings: Settings::default(),
            waveform_map: HashMap::new(),
            waveform_channel: HashMap::new(),
        }
    }
}

impl MantleApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }

    fn sort_bulbs<'a>(&self, mut bulbs: Vec<&'a BulbInfo>) -> Vec<&'a BulbInfo> {
        bulbs.sort_by(|a, b| {
            let group_a = a.group_label();
            let group_b = b.group_label();
            let name_a = a.name_label();
            let name_b = b.name_label();
            group_a.cmp(&group_b).then(name_a.cmp(&name_b))
        });
        bulbs
    }

    fn display_device(
        &mut self,
        ui: &mut Ui,
        device: &DeviceInfo,
        bulbs: &MutexGuard<HashMap<u64, BulbInfo>>,
    ) {
        let color = match device {
            DeviceInfo::Bulb(bulb) => {
                if let Some(s) = bulb.name.data.as_ref().and_then(|s| s.to_str().ok()) {
                    ui.label(RichText::new(s).size(14.0));
                }
                bulb.get_color().cloned()
            }
            DeviceInfo::Group(group) => {
                if let Ok(s) = group.label.cstr().to_str() {
                    if *group == self.mgr.all {
                        ui.label(RichText::new(s).size(16.0).strong().underline());
                    } else {
                        ui.label(RichText::new(s).size(16.0).strong());
                    }
                }
                Some(self.mgr.get_avg_group_color(group, bulbs))
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
                    let mut after_color =
                        self.display_color_controls(ui, device, color.unwrap_or(default_hsbk()));
                    ui.horizontal(|ui| {
                        after_color = handle_eyedropper(self, ui, device).unwrap_or(after_color);
                        after_color = handle_screencap(self, ui, device).unwrap_or(after_color);
                    });
                    if before_color != after_color.next {
                        match device {
                            DeviceInfo::Bulb(bulb) => {
                                if let Err(e) = self.mgr.set_color(
                                    &&**bulb,
                                    after_color.next,
                                    after_color.duration,
                                ) {
                                    log::error!("Error setting color: {}", e);
                                }
                            }
                            DeviceInfo::Group(group) => {
                                if let Err(e) = self.mgr.set_group_color(
                                    group,
                                    after_color.next,
                                    bulbs,
                                    after_color.duration,
                                ) {
                                    log::error!("Error setting group color: {}", e);
                                }
                            }
                        }
                    }
                } else {
                    ui.label(format!("No color data: {:?}", color));
                }
            });
        });
        ui.separator();
    }

    fn display_color_controls(&self, ui: &mut Ui, device: &DeviceInfo, color: HSBK) -> DeltaColor {
        let HSBK {
            mut hue,
            mut saturation,
            mut brightness,
            mut kelvin,
        } = color;
        hsbk_sliders(
            ui,
            &mut hue,
            &mut saturation,
            &mut brightness,
            &mut kelvin,
            device,
        );
        DeltaColor {
            next: HSBK {
                hue,
                saturation,
                brightness,
                kelvin,
            },
            duration: None,
        }
    }

    fn file_menu_button(&mut self, ui: &mut Ui) {
        let close_shortcut = egui::KeyboardShortcut::new(Modifiers::CTRL, egui::Key::Q);
        let refresh_shortcut = egui::KeyboardShortcut::new(Modifiers::NONE, egui::Key::F5);
        if ui.input_mut(|i| i.consume_shortcut(&close_shortcut)) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if ui.input_mut(|i| i.consume_shortcut(&refresh_shortcut)) {
            if let Err(e) = self.mgr.refresh() {
                log::error!("Error refreshing manager: {}", e);
            }
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
                if let Err(e) = self.mgr.refresh() {
                    log::error!("Error refreshing manager: {}", e);
                }
                ui.close_menu();
            }
            if ui.add(egui::Button::new("Settings")).clicked() {
                self.show_settings = true;
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

    fn update_ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                self.file_menu_button(ui);
                self.help_menu_button(ui);
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                let bulbs = self.mgr.bulbs.clone();
                let bulbs = bulbs.lock();
                let mut seen_groups = HashSet::<String>::new();
                ui.vertical(|ui| {
                    if let Ok(bulbs) = bulbs {
                        self.display_device(ui, &DeviceInfo::Group(self.mgr.all.clone()), &bulbs);
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
                            self.display_device(
                                ui,
                                &DeviceInfo::Bulb(Box::new(bulb.clone())),
                                &bulbs,
                            );
                        }
                    }
                });
            });
        });
    }

    fn show_about_window(&mut self, ctx: &egui::Context) {
        if self.show_about {
            egui::Window::new("About")
                .default_width(ABOUT_WINDOW_SIZE[0])
                .default_height(ABOUT_WINDOW_SIZE[1])
                .open(&mut self.show_about)
                .resizable([true, false])
                .show(ctx, |ui| {
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

impl eframe::App for MantleApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(debug_assertions)]
        puffin::GlobalProfiler::lock().new_frame();
        if Instant::now() - self.mgr.last_discovery
            > Duration::from_millis(self.settings.refresh_rate_ms)
        {
            self.mgr.discover().expect("Failed to discover bulbs");
        }
        if let Err(e) = self.mgr.refresh() {
            log::error!("Error refreshing manager: {}", e);
        }
        self.update_ui(_ctx);
        self.show_about_window(_ctx);
        self.settings_ui(_ctx);
    }
}
