#![cfg_attr(
    all(target_os = "windows", not(debug_assertions),),
    windows_subsystem = "windows"
)]
// Hide console window on Release
use device_query::{DeviceQuery, DeviceState};
use eframe::egui::{self, Color32, Modifiers, RichText, Ui, Vec2};
use image::GenericImageView;
use lifx_core::HSBK;
use log::LevelFilter;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;
use log4rs::{
    append::{console::ConsoleAppender, file::FileAppender},
    filter::threshold::ThresholdFilter,
};
use mantle::color::{kelvin_to_rgb, DeltaColor, HSBK32};
use mantle::products::TemperatureRange;
use mantle::screencap::FollowType;
use mantle::{color_slider, ScreencapManager};
use serde::{Deserialize, Serialize};
use std::ops::RangeInclusive;
use std::sync::mpsc;
use std::thread;
use std::{
    collections::{HashMap, HashSet},
    sync::MutexGuard,
    time::{Duration, Instant},
};

use mantle::{
    capitalize_first_letter, color::default_hsbk, device_info::DeviceInfo, display_color_circle,
    toggle_button, BulbInfo, Manager,
};

const MAIN_WINDOW_SIZE: [f32; 2] = [320.0, 800.0];
const ABOUT_WINDOW_SIZE: [f32; 2] = [320.0, 480.0];
const MIN_WINDOW_SIZE: [f32; 2] = [300.0, 220.0];
const LIFX_RANGE: std::ops::RangeInclusive<u16> = 0..=u16::MAX;
const KELVIN_RANGE: TemperatureRange = TemperatureRange {
    min: 2500,
    max: 9000,
};
const REFRESH_RATE: Duration = Duration::from_secs(10);
const FOLLOW_RATE: Duration = Duration::from_millis(500);
const ICON: &[u8; 1751] = include_bytes!("../res/logo32.png");
const EYEDROPPER_ICON: &[u8; 238] = include_bytes!("../res/icons/color-picker.png");
const MONITOR_ICON: &[u8; 204] = include_bytes!("../res/icons/device-desktop.png");

fn main() -> eframe::Result {
    start_puffin_server();
    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build("log/output.log")
        .expect("Failed to create log file appender");

    let console = ConsoleAppender::builder().build();

    let config = Config::builder()
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(LevelFilter::Info)))
                .build("logfile", Box::new(logfile)),
        )
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(LevelFilter::Debug)))
                .build("stdout", Box::new(console)),
        )
        .build(
            Root::builder()
                .appender("logfile")
                .appender("stdout")
                .build(LevelFilter::Debug),
        )
        .expect("Failed to create log config");

    log4rs::init_config(config).expect("Failed to initialize log4rs");

    let icon = image::load_from_memory(ICON).expect("Failed to load icon");
    let icon = egui::IconData {
        rgba: icon.to_rgba8().into_raw(),
        width: icon.width(),
        height: icon.height(),
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(MAIN_WINDOW_SIZE)
            .with_min_inner_size(MIN_WINDOW_SIZE)
            .with_icon(icon),
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

#[derive(Debug, Clone)]
struct RunningWaveform {
    active: bool,
    last_update: Instant,
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
struct MantleApp {
    #[serde(skip)]
    mgr: Manager,
    #[serde(skip)]
    screen_manager: ScreencapManager,
    show_about: bool,
    show_eyedropper: bool,
    #[serde(skip)]
    waveform_map: HashMap<u64, RunningWaveform>,
    #[serde(skip)]
    waveform_trx: HashMap<u64, (mpsc::Sender<HSBK>, mpsc::Receiver<HSBK>)>,
}

impl Default for MantleApp {
    fn default() -> Self {
        let mgr = Manager::new().expect("Failed to create manager");
        let screen_manager = ScreencapManager::new().expect("Failed to create screen manager");
        Self {
            mgr,
            screen_manager,
            show_about: false,
            show_eyedropper: false,
            waveform_map: HashMap::new(),
            waveform_trx: HashMap::new(),
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
                    let mut after_color =
                        self.display_color_controls(ui, device, color.unwrap_or(default_hsbk()));
                    ui.horizontal(|ui| {
                        after_color = handle_eyedropper(self, ui).unwrap_or(after_color);
                        after_color = handle_screencap(self, ui, device).unwrap_or(after_color);
                        // destruct the color to get the after color
                    });
                    if before_color != after_color.next {
                        match device {
                            DeviceInfo::Bulb(bulb) => {
                                if let Err(e) =
                                    self.mgr
                                        .set_color(bulb, after_color.next, after_color.duration)
                                {
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
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label("Hue");
                color_slider(ui, &mut hue, LIFX_RANGE, "Hue", |v| {
                    HSBK32 {
                        hue: v as u32,
                        saturation: u32::MAX,
                        brightness: u32::MAX,
                        kelvin: 0,
                    }
                    .into()
                });
            });
            ui.horizontal(|ui| {
                ui.label("Saturation");
                color_slider(ui, &mut saturation, LIFX_RANGE, "Saturation", |v| {
                    let color_value = (u16::MAX - v).max(0) / u8::MAX as u16;
                    Color32::from_gray(color_value as u8)
                });
            });
            ui.horizontal(|ui| {
                ui.label("Brightness");
                color_slider(ui, &mut brightness, LIFX_RANGE, "Brightness", |v| {
                    let color_value = v.min(u16::MAX) / u8::MAX as u16;
                    Color32::from_gray(color_value as u8)
                });
            });
            ui.horizontal(|ui| {
                ui.label("Kelvin");
                match device {
                    DeviceInfo::Bulb(bulb) => {
                        if let Some(range) = bulb.features.temperature_range.as_ref() {
                            if range.min != range.max {
                                color_slider(
                                    ui,
                                    &mut kelvin,
                                    RangeInclusive::new(range.min as u16, range.max as u16),
                                    "Kelvin",
                                    |v| {
                                        let temp = (((v as f32 / u16::MAX as f32)
                                            * (range.max - range.min) as f32)
                                            + range.min as f32)
                                            as u16;
                                        kelvin_to_rgb(temp).into()
                                    },
                                );
                            } else {
                                ui.label(format!("{}K", range.min));
                            }
                        }
                    }
                    DeviceInfo::Group(_) => {
                        color_slider(
                            ui,
                            &mut kelvin,
                            RangeInclusive::new(KELVIN_RANGE.min as u16, KELVIN_RANGE.max as u16),
                            "Kelvin",
                            |v| {
                                let temp = (((v as f32 / u16::MAX as f32)
                                    * (KELVIN_RANGE.max - KELVIN_RANGE.min) as f32)
                                    + KELVIN_RANGE.min as f32)
                                    as u16;
                                kelvin_to_rgb(temp).into()
                            },
                        );
                    }
                }
            });
        });
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

fn handle_eyedropper(app: &mut MantleApp, ui: &mut Ui) -> Option<DeltaColor> {
    let mut color: Option<HSBK> = None;
    if ui
        .add(
            egui::Button::image(
                egui::Image::from_bytes("eyedropper", EYEDROPPER_ICON)
                    .fit_to_exact_size(Vec2::new(15., 15.)),
            )
            .sense(egui::Sense::click()),
        )
        .clicked()
    {
        app.show_eyedropper = !app.show_eyedropper;
    }
    if app.show_eyedropper {
        let screencap = ScreencapManager::new().unwrap();
        ui.ctx().output_mut(|out| {
            out.cursor_icon = egui::CursorIcon::Crosshair;
        });
        let device_state = DeviceState::new();
        let mouse = device_state.get_mouse();
        if mouse.button_pressed[1] {
            let position = mouse.coords;
            color = Some(screencap.from_click(position.0, position.1));
            app.show_eyedropper = false;
        }
    }
    color.map(|color| DeltaColor {
        next: color,
        duration: None,
    })
}

fn handle_screencap(app: &mut MantleApp, ui: &mut Ui, device: &DeviceInfo) -> Option<DeltaColor> {
    puffin::profile_function!();
    let mut color: Option<HSBK> = None;
    // get tx and rx for the device
    if let Some((_tx, rx)) = app.waveform_trx.get(&device.id()) {
        let follow_state: &mut RunningWaveform =
            app.waveform_map
                .entry(device.id())
                .or_insert(RunningWaveform {
                    active: false,
                    last_update: Instant::now(),
                });
        if follow_state.active && (Instant::now() - follow_state.last_update > FOLLOW_RATE) {
            if let Ok(computed_color) = rx.try_recv() {
                color = Some(computed_color);
                follow_state.last_update = Instant::now();
            }
        }
    } else {
        let (tx, rx) = mpsc::channel();
        app.waveform_trx.insert(device.id(), (tx, rx));
    }

    if ui
        .add(
            egui::Button::image(
                egui::Image::from_bytes("monitor", MONITOR_ICON)
                    .fit_to_exact_size(Vec2::new(15., 15.)),
            )
            .sense(egui::Sense::click()),
        )
        .clicked()
    {
        if let Some(waveform) = app.waveform_map.get_mut(&device.id()) {
            waveform.active = !waveform.active;
        } else {
            let running_waveform = RunningWaveform {
                active: true,
                last_update: Instant::now(),
            };
            app.waveform_map
                .insert(device.id(), running_waveform.clone());
        }
        // if the waveform is active, we need to spawn a thread to get the color
        if app.waveform_map[&device.id()].active {
            let screen_manager = app.screen_manager.clone(); // Clone if necessary
            let tx = app.waveform_trx.get(&device.id()).unwrap().0.clone();
            thread::spawn(move || loop {
                let avg_color = screen_manager.avg_color(FollowType::All);
                if let Err(err) = tx.send(avg_color) {
                    eprintln!("Failed to send color data: {}", err);
                }
                thread::sleep(Duration::from_millis(100));
            });
        } else {
            // kill thread
        }
    }

    color.map(|color| DeltaColor {
        next: color,
        duration: Some((FOLLOW_RATE.as_millis() / 2) as u32),
    })
}

impl eframe::App for MantleApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        puffin::GlobalProfiler::lock().new_frame();
        // puffin_egui::show_viewport_if_enabled(_ctx);
        if Instant::now() - self.mgr.last_discovery > REFRESH_RATE {
            self.mgr.discover().expect("Failed to discover bulbs");
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

fn start_puffin_server() {
    puffin::set_scopes_on(true); // tell puffin to collect data

    match puffin_http::Server::new("127.0.0.1:8585") {
        Ok(puffin_server) => {
            eprintln!("Run:  cargo install puffin_viewer && puffin_viewer --url 127.0.0.1:8585");

            std::process::Command::new("puffin_viewer")
                .arg("--url")
                .arg("127.0.0.1:8585")
                .spawn()
                .ok();

            // We can store the server if we want, but in this case we just want
            // it to keep running. Dropping it closes the server, so let's not drop it!
            #[allow(clippy::mem_forget)]
            std::mem::forget(puffin_server);
        }
        Err(err) => {
            eprintln!("Failed to start puffin server: {err}");
        }
    };
}
