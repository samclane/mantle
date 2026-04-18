use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex, MutexGuard,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use crate::{
    audio::AudioManager,
    capitalize_first_letter,
    color::{default_hsbk, DeltaColor},
    device_info::DeviceInfo,
    display_color_circle,
    listener::input_listener::InputListener,
    products::get_product_name,
    scenes::Scene,
    screencap::{RegionCaptureTarget, ScreenSubregion},
    settings::Settings,
    shortcut::{KeyboardShortcutAction, ShortcutManager},
    toggle_button,
    ui::{
        color_wheel, handle_audio, handle_eyedropper, handle_screencap, hsbk_sliders,
        infrared_slider, matrix_grid, render_capture_target, rgb_input, zone_strip,
    },
    BulbInfo, LifxManager, ScreencapManager,
};

use eframe::egui::{self, Color32, Direction, Modifiers, RichText, Stroke, Ui, Vec2};
use egui::Align2;
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use lifx_core::{ApplicationRequest, HSBK};
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::TrayIcon;

#[cfg(windows)]
extern "system" {
    fn ShowWindow(hWnd: isize, nCmdShow: i32) -> i32;
    fn SetForegroundWindow(hWnd: isize) -> i32;
    fn FindWindowW(lpClassName: *const u16, lpWindowName: *const u16) -> isize;
}

#[cfg(windows)]
fn win32_show_mantle_window() {
    let title: Vec<u16> = "Mantle".encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
        if hwnd != 0 {
            ShowWindow(hwnd, 5); // SW_SHOW
            SetForegroundWindow(hwnd);
        }
    }
}

enum TrayAction {
    Show,
    Hide,
    Quit,
}

// UI and window size constants
pub const MAIN_WINDOW_SIZE: [f32; 2] = [420.0, 800.0];
pub const ABOUT_WINDOW_SIZE: [f32; 2] = [320.0, 480.0];
pub const MIN_WINDOW_SIZE: [f32; 2] = [380.0, 220.0];

// Icon data
pub const ICON: &[u8; 1751] = include_bytes!("../res/logo32.png");
pub const EYEDROPPER_ICON: &[u8; 238] = include_bytes!("../res/icons/color-picker.png");
pub const MONITOR_ICON: &[u8; 204] = include_bytes!("../res/icons/device-desktop.png");
pub const SUBREGION_ICON: &[u8; 218] = include_bytes!("../res/icons/square.png");
pub const AUDIO_ICON: &[u8; 225] = include_bytes!("../res/icons/device-speaker.png");
pub const SCREENSHOT_ICON: &[u8] = include_bytes!("../res/icons/screenshot.png");

#[derive(Debug, Clone, PartialEq)]
pub enum WaveformMode {
    Screencap,
    Audio,
}

#[derive(Debug, Clone)]
pub struct WaveformTracker {
    pub active: bool,
    pub last_update: Instant,
    pub mode: WaveformMode,
    pub region: RegionCaptureTarget,
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
    pub audio_manager: AudioManager,
    #[serde(skip)]
    pub input_listener: InputListener,
    #[serde(skip)]
    pub lighting_manager: LifxManager,
    pub new_scene: Scene,
    #[serde(skip)]
    pub screen_manager: ScreencapManager,
    pub settings: Settings,
    #[serde(skip)]
    pub shortcut_handle: Option<JoinHandle<()>>,
    #[serde(skip)]
    pub shortcut_manager: ShortcutManager,
    #[serde(skip)]
    pub renaming_device: Option<u64>,
    #[serde(skip)]
    pub rename_buffer: String,
    pub search_query: String,
    pub show_about: bool,
    pub show_audio_debug: bool,
    pub show_eyedropper: HashMap<u64, bool>,
    pub show_settings: bool,
    pub show_subregion: HashMap<u64, bool>,
    pub subregion_points: HashMap<u64, Arc<Mutex<ScreenSubregion>>>,
    #[serde(skip)]
    pub toasts: Toasts,
    #[serde(skip)]
    window_visible: Arc<AtomicBool>,
    #[serde(skip)]
    pub tray_icon: Option<TrayIcon>,
    #[serde(skip)]
    tray_event_rx: Option<mpsc::Receiver<TrayAction>>,
    #[serde(skip)]
    pub monitor_preview_textures: HashMap<u32, egui::TextureHandle>,
    #[serde(skip)]
    pub selected_zones: HashMap<u64, HashSet<usize>>,
    #[serde(skip)]
    pub waveform_channel: ColorChannel,
    #[serde(skip)]
    pub waveform_map: HashMap<u64, WaveformTracker>,
    #[serde(skip)]
    pub last_refresh: Instant,
    #[serde(skip)]
    pub last_schedule_check: Instant,
}

impl Default for MantleApp {
    fn default() -> Self {
        let input_listener = InputListener::new();
        let shortcut_manager = ShortcutManager::new(input_listener.clone());
        let lifx_manager = LifxManager::new().expect("Failed to create manager");
        let shortcut_handle = Some(shortcut_manager.start(lifx_manager.clone()));
        Self {
            lighting_manager: lifx_manager,
            screen_manager: ScreencapManager::new().expect("Failed to create screen manager"),
            input_listener,
            shortcut_manager,
            shortcut_handle,
            renaming_device: None,
            rename_buffer: String::new(),
            search_query: String::new(),
            show_about: false,
            show_settings: false,
            show_eyedropper: HashMap::new(),
            show_subregion: HashMap::new(),
            subregion_points: HashMap::new(),
            settings: Settings::default(),
            selected_zones: HashMap::new(),
            waveform_map: HashMap::new(),
            waveform_channel: HashMap::new(),
            monitor_preview_textures: HashMap::new(),
            new_scene: Scene::new(vec![], t!("scenes.unnamed").to_string()),
            toasts: Toasts::new()
                .anchor(Align2::CENTER_TOP, (0.0, 10.0))
                .direction(Direction::TopDown),
            window_visible: Arc::new(AtomicBool::new(true)),
            tray_icon: None,
            tray_event_rx: None,
            audio_manager: AudioManager::default(),
            show_audio_debug: false,
            last_refresh: Instant::now(),
            last_schedule_check: Instant::now(),
        }
    }
}

impl MantleApp {
    fn setup_tray_icon(&mut self, ctx: &egui::Context) {
        let menu = Menu::new();
        let show_item = MenuItem::new(&*t!("app.tray.show_hide"), true, None);
        let toggle_item = MenuItem::new(&*t!("app.tray.toggle_power"), true, None);
        let quit_item = MenuItem::new(&*t!("app.tray.quit"), true, None);
        let _ = menu.append(&show_item);
        let _ = menu.append(&toggle_item);
        let _ = menu.append(&quit_item);

        let icon_data = image::load_from_memory(ICON)
            .map(|img| {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                tray_icon::Icon::from_rgba(rgba.into_raw(), w, h).ok()
            })
            .ok()
            .flatten();

        if let Some(icon) = icon_data {
            match tray_icon::TrayIconBuilder::new()
                .with_menu(Box::new(menu))
                .with_tooltip(&*t!("app.tray.tooltip"))
                .with_icon(icon)
                .build()
            {
                Ok(tray) => {
                    self.tray_icon = Some(tray);

                    let (tx, rx) = mpsc::channel();
                    self.tray_event_rx = Some(rx);

                    let show_id = show_item.id().clone();
                    let toggle_id = toggle_item.id().clone();
                    let quit_id = quit_item.id().clone();
                    let ctx = ctx.clone();
                    let visible = self.window_visible.clone();
                    let lifx = self.lighting_manager.clone();

                    std::thread::spawn(move || {
                        while let Ok(event) = MenuEvent::receiver().recv() {
                            if event.id == show_id {
                                if visible.load(Ordering::SeqCst) {
                                    let _ = tx.send(TrayAction::Hide);
                                    ctx.request_repaint();
                                } else {
                                    #[cfg(windows)]
                                    win32_show_mantle_window();
                                    let _ = tx.send(TrayAction::Show);
                                    ctx.request_repaint();
                                }
                            } else if event.id == toggle_id {
                                if let Err(e) = lifx.toggle_power() {
                                    log::error!("Failed to toggle power: {}", e);
                                }
                            } else if event.id == quit_id {
                                #[cfg(windows)]
                                if !visible.load(Ordering::SeqCst) {
                                    win32_show_mantle_window();
                                }
                                let _ = tx.send(TrayAction::Quit);
                                ctx.request_repaint();
                            }
                        }
                    });
                }
                Err(e) => log::error!("Failed to create tray icon: {}", e),
            }
        }
    }

    fn handle_tray_events(&mut self, ctx: &egui::Context) {
        if let Some(ref rx) = self.tray_event_rx {
            while let Ok(action) = rx.try_recv() {
                match action {
                    TrayAction::Show => {
                        self.window_visible.store(true, Ordering::SeqCst);
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    }
                    TrayAction::Hide => {
                        self.window_visible.store(false, Ordering::SeqCst);
                        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    }
                    TrayAction::Quit => {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
            }
        }
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::configure_theme(&cc.egui_ctx);
        Self::configure_fonts(&cc.egui_ctx);

        if let Some(storage) = cc.storage {
            let mut app =
                eframe::get_value::<MantleApp>(storage, eframe::APP_KEY).unwrap_or_default();
            rust_i18n::set_locale(&app.settings.locale);
            let failures: Vec<KeyboardShortcutAction> = app
                .settings
                .custom_shortcuts
                .clone()
                .into_iter()
                .filter_map(|shortcut| {
                    app.shortcut_manager
                        .add_action(shortcut.clone())
                        .err()
                        .map(|e| {
                            log::error!("Failed to add shortcut action: {}", e);
                            shortcut
                        })
                })
                .collect();

            app.audio_manager
                .build_input_stream(&app.settings.audio_buffer_size)
                .unwrap();

            if !failures.is_empty() {
                app.error_toast(&t!(
                    "error.shortcut_add_failed",
                    count = failures.len(),
                    details = format!("{:?}", failures)
                ));
            }
            app.sync_auto_launch_state();
            app.setup_tray_icon(&cc.egui_ctx);
            return app;
        }
        let mut app = Self::default();
        app.sync_auto_launch_state();
        app.setup_tray_icon(&cc.egui_ctx);
        app
    }

    fn configure_fonts(ctx: &egui::Context) {
        let cjk_font_path = if cfg!(target_os = "windows") {
            Some(std::path::PathBuf::from("C:\\Windows\\Fonts\\msyh.ttc"))
        } else if cfg!(target_os = "macos") {
            Some(std::path::PathBuf::from(
                "/System/Library/Fonts/PingFang.ttc",
            ))
        } else {
            [
                "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
                "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            ]
            .iter()
            .map(std::path::PathBuf::from)
            .find(|p| p.exists())
        };

        if let Some(path) = cjk_font_path {
            if let Ok(data) = std::fs::read(&path) {
                let mut fonts = egui::FontDefinitions::default();
                fonts
                    .font_data
                    .insert("cjk".to_owned(), egui::FontData::from_owned(data).into());
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .push("cjk".to_owned());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .push("cjk".to_owned());
                ctx.set_fonts(fonts);
            }
        }
    }

    fn configure_theme(ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();

        visuals.panel_fill = Color32::from_rgb(22, 22, 30);
        visuals.window_fill = Color32::from_rgb(28, 28, 38);
        visuals.extreme_bg_color = Color32::from_rgb(16, 16, 22);
        visuals.faint_bg_color = Color32::from_rgb(32, 32, 44);

        let widget_rounding = egui::Rounding::same(6.0);
        visuals.widgets.noninteractive.rounding = widget_rounding;
        visuals.widgets.inactive.rounding = widget_rounding;
        visuals.widgets.hovered.rounding = widget_rounding;
        visuals.widgets.active.rounding = widget_rounding;
        visuals.widgets.open.rounding = widget_rounding;
        visuals.window_rounding = egui::Rounding::same(10.0);
        visuals.menu_rounding = egui::Rounding::same(8.0);

        visuals.selection.bg_fill = Color32::from_rgb(180, 120, 30);
        visuals.selection.stroke = Stroke::new(1.0, Color32::from_rgb(220, 160, 50));

        visuals.widgets.inactive.bg_fill = Color32::from_rgb(40, 40, 55);
        visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(35, 35, 48);
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(50, 50, 68);
        visuals.widgets.active.bg_fill = Color32::from_rgb(60, 60, 80);

        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(55, 55, 75));
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(100, 100, 140));
        visuals.widgets.active.bg_stroke = Stroke::new(1.5, Color32::from_rgb(180, 120, 30));

        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.slider_rail_height = 14.0;
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        ctx.set_style(style);
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

    fn get_device_display_color(
        &mut self,
        ui: &mut egui::Ui,
        device: &DeviceInfo,
        bulbs: &MutexGuard<HashMap<u64, BulbInfo>>,
    ) -> Option<HSBK> {
        match device {
            DeviceInfo::Bulb(bulb) => {
                let device_id = bulb.target;
                let is_renaming = self.renaming_device == Some(device_id);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        let elapsed = bulb.last_seen.elapsed();
                        let is_online = elapsed < Duration::from_secs(30);
                        let dot_color = if is_online {
                            Color32::from_rgb(80, 200, 120)
                        } else {
                            Color32::from_rgb(200, 80, 80)
                        };
                        let (dot_resp, painter) =
                            ui.allocate_painter(Vec2::new(8.0, 14.0), egui::Sense::hover());
                        painter.circle_filled(dot_resp.rect.center(), 3.5, dot_color);
                        let tooltip = if is_online {
                            t!(
                                "devices.online",
                                seconds = format!("{:.0}", elapsed.as_secs_f32())
                            )
                        } else {
                            t!(
                                "devices.offline",
                                seconds = format!("{:.0}", elapsed.as_secs_f32())
                            )
                        };
                        dot_resp.on_hover_text(tooltip);

                        if is_renaming {
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.rename_buffer)
                                    .desired_width(120.0),
                            );
                            if resp.lost_focus() {
                                if let Ok(cstr) = std::ffi::CString::new(self.rename_buffer.clone())
                                {
                                    let label = lifx_core::LifxString::new(&cstr);
                                    if let Err(e) = self.lighting_manager.set_label(&&**bulb, label)
                                    {
                                        log::error!("Failed to rename device: {}", e);
                                    }
                                }
                                self.renaming_device = None;
                            }
                        } else if let Some(s) =
                            bulb.name.data.as_ref().and_then(|s| s.to_str().ok())
                        {
                            let name_resp = ui.add(
                                egui::Label::new(
                                    RichText::new(s)
                                        .size(14.0)
                                        .color(Color32::from_rgb(200, 200, 220)),
                                )
                                .sense(egui::Sense::click()),
                            );
                            if name_resp.double_clicked() {
                                self.renaming_device = Some(device_id);
                                self.rename_buffer = s.to_string();
                            }
                            name_resp.on_hover_text(t!("devices.rename_hint").to_string());
                        }
                    });
                    if let Some(product_name) = get_product_name(bulb.model.data.as_ref()) {
                        ui.label(
                            RichText::new(product_name)
                                .size(11.0)
                                .color(Color32::from_rgb(140, 140, 160)),
                        );
                    }
                });
                bulb.get_color().cloned()
            }
            DeviceInfo::Group(group) => {
                if let Ok(s) = group.label.cstr().to_str() {
                    if *group == self.lighting_manager.all_bulbs_group {
                        ui.label(
                            RichText::new(s)
                                .size(17.0)
                                .strong()
                                .color(Color32::from_rgb(220, 220, 240)),
                        );
                    } else {
                        ui.label(
                            RichText::new(s)
                                .size(16.0)
                                .strong()
                                .color(Color32::from_rgb(210, 210, 230)),
                        );
                    }
                }
                Some(self.lighting_manager.get_avg_group_color(group, bulbs))
            }
        }
    }

    fn render_device_controls(
        &mut self,
        ui: &mut egui::Ui,
        device: &DeviceInfo,
        color_opt: Option<HSBK>,
        bulbs: &mut MutexGuard<HashMap<u64, BulbInfo>>,
    ) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(t!("controls.power").to_string())
                        .size(12.0)
                        .color(Color32::from_rgb(160, 160, 180)),
                );
                toggle_button(
                    ui,
                    &self.lighting_manager,
                    device,
                    Vec2::new(1.0, 1.0),
                    bulbs,
                )
                .on_hover_text(t!("controls.toggle_power").to_string());
            });

            let is_multizone = matches!(device, DeviceInfo::Bulb(b) if b.is_multizone());
            let is_matrix = matches!(device, DeviceInfo::Bulb(b) if b.is_matrix());
            let has_zones = is_multizone || is_matrix;
            let device_id = device.id();

            let selected = self
                .selected_zones
                .get(&device_id)
                .cloned()
                .unwrap_or_default();

            let slider_color = if has_zones && !selected.is_empty() {
                if let DeviceInfo::Bulb(bulb) = device {
                    let first_selected = *selected.iter().min().unwrap();
                    bulb.get_zone_color(first_selected).cloned()
                } else {
                    color_opt
                }
            } else {
                color_opt
            };

            if let Some(before_color) = slider_color {
                ui.add_space(2.0);
                let mut after_color = self.display_color_controls(ui, device, before_color);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    after_color = handle_eyedropper(self, ui, device).unwrap_or(after_color);
                    after_color = handle_screencap(self, ui, device).unwrap_or(after_color);
                    after_color = handle_audio(self, ui, device).unwrap_or(after_color);
                });
                render_capture_target(self, ui, device);

                let has_infrared = match device {
                    DeviceInfo::Bulb(bulb) => bulb.features.infrared == Some(true),
                    DeviceInfo::Group(group) => group
                        .get_bulbs(bulbs)
                        .iter()
                        .any(|b| b.features.infrared == Some(true)),
                };
                if has_infrared {
                    ui.add_space(4.0);
                    let mut ir_brightness = match device {
                        DeviceInfo::Bulb(bulb) => bulb.infrared.data.unwrap_or(0),
                        DeviceInfo::Group(group) => {
                            let ir_bulbs: Vec<&BulbInfo> = group
                                .get_bulbs(bulbs)
                                .into_iter()
                                .filter(|b| b.features.infrared == Some(true))
                                .collect();
                            if ir_bulbs.is_empty() {
                                0
                            } else {
                                let sum: u32 = ir_bulbs
                                    .iter()
                                    .map(|b| b.infrared.data.unwrap_or(0) as u32)
                                    .sum();
                                (sum / ir_bulbs.len() as u32) as u16
                            }
                        }
                    };
                    let before_ir = ir_brightness;
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(t!("controls.infrared").to_string())
                                .size(12.0)
                                .color(Color32::from_rgb(160, 160, 180)),
                        );
                        infrared_slider(ui, &mut ir_brightness)
                            .on_hover_text(t!("slider.infrared_hover").to_string());
                    });
                    if ir_brightness != before_ir {
                        match device {
                            DeviceInfo::Bulb(bulb) => {
                                if let Err(e) =
                                    self.lighting_manager.set_infrared(&&**bulb, ir_brightness)
                                {
                                    log::error!("Error setting infrared: {}", e);
                                    self.error_toast(&t!(
                                        "error.set_infrared",
                                        error = e.to_string()
                                    ));
                                }
                            }
                            DeviceInfo::Group(group) => {
                                if let Err(e) = self.lighting_manager.set_group_infrared(
                                    group,
                                    bulbs,
                                    ir_brightness,
                                ) {
                                    log::error!("Error setting group infrared: {}", e);
                                    self.error_toast(&t!(
                                        "error.set_infrared",
                                        error = e.to_string()
                                    ));
                                }
                            }
                        }
                    }
                }

                if is_multizone {
                    if let DeviceInfo::Bulb(bulb) = device {
                        if let Some(zones) = bulb.get_zone_colors() {
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(t!("controls.zones").to_string())
                                        .size(12.0)
                                        .color(Color32::from_rgb(160, 160, 180)),
                                );
                                if ui
                                    .small_button(
                                        if selected.len() == zones.len() && !zones.is_empty() {
                                            t!("controls.deselect_all").to_string()
                                        } else {
                                            t!("controls.select_all").to_string()
                                        },
                                    )
                                    .on_hover_text(t!("controls.select_zones_hover").to_string())
                                    .clicked()
                                {
                                    let new_sel =
                                        if selected.len() == zones.len() && !zones.is_empty() {
                                            HashSet::new()
                                        } else {
                                            (0..zones.len()).collect()
                                        };
                                    self.selected_zones.insert(device_id, new_sel);
                                }
                            });
                            let current_selected = self
                                .selected_zones
                                .get(&device_id)
                                .cloned()
                                .unwrap_or_default();
                            let new_selected = zone_strip(ui, zones, &current_selected);
                            self.selected_zones.insert(device_id, new_selected);

                            ui.add_space(2.0);
                            if ui
                                .small_button(t!("controls.apply_gradient").to_string())
                                .on_hover_text(t!("controls.apply_gradient_hover").to_string())
                                .clicked()
                            {
                                let zone_count = zones.len();
                                if zone_count > 0 {
                                    let start_hue: f32 = 0.0;
                                    let end_hue: f32 = 54613.0;
                                    let duration = after_color.duration.unwrap_or(0);
                                    for i in 0..zone_count {
                                        let t = i as f32 / (zone_count - 1).max(1) as f32;
                                        let zone_hue =
                                            (start_hue + (end_hue - start_hue) * t) as u16;
                                        let zone_color = HSBK {
                                            hue: zone_hue,
                                            saturation: after_color.next.saturation,
                                            brightness: after_color.next.brightness,
                                            kelvin: after_color.next.kelvin,
                                        };
                                        let apply = if i == zone_count - 1 {
                                            ApplicationRequest::Apply
                                        } else {
                                            ApplicationRequest::NoApply
                                        };
                                        if let Err(e) = self.lighting_manager.set_color_zones(
                                            &&**bulb, i as u8, i as u8, zone_color, duration, apply,
                                        ) {
                                            log::error!("Error setting gradient zone: {}", e);
                                            break;
                                        }
                                    }
                                    self.success_toast(&t!("controls.gradient_applied"));
                                }
                            }
                        }
                    }
                }

                if is_matrix {
                    if let DeviceInfo::Bulb(bulb) = device {
                        if let Some(zones) = bulb.get_zone_colors() {
                            let grid_width = bulb.get_matrix_width();
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(t!("controls.matrix").to_string())
                                        .size(12.0)
                                        .color(Color32::from_rgb(160, 160, 180)),
                                );
                                if ui
                                    .small_button(
                                        if selected.len() == zones.len() && !zones.is_empty() {
                                            t!("controls.deselect_all").to_string()
                                        } else {
                                            t!("controls.select_all").to_string()
                                        },
                                    )
                                    .on_hover_text(t!("controls.select_cells_hover").to_string())
                                    .clicked()
                                {
                                    let new_sel =
                                        if selected.len() == zones.len() && !zones.is_empty() {
                                            HashSet::new()
                                        } else {
                                            (0..zones.len()).collect()
                                        };
                                    self.selected_zones.insert(device_id, new_sel);
                                }
                            });
                            let current_selected = self
                                .selected_zones
                                .get(&device_id)
                                .cloned()
                                .unwrap_or_default();
                            let new_selected =
                                matrix_grid(ui, zones, grid_width, &current_selected);
                            self.selected_zones.insert(device_id, new_selected);

                            ui.add_space(2.0);
                            if ui
                                .small_button(t!("controls.apply_gradient").to_string())
                                .on_hover_text(
                                    t!("controls.apply_matrix_gradient_hover").to_string(),
                                )
                                .clicked()
                            {
                                let zone_count = zones.len();
                                if zone_count > 0 {
                                    let start_hue: f32 = 0.0;
                                    let end_hue: f32 = 54613.0;
                                    let duration = after_color.duration.unwrap_or(0);
                                    let mut updates = HashMap::new();
                                    for i in 0..zone_count {
                                        let t = i as f32 / (zone_count - 1).max(1) as f32;
                                        let zone_hue =
                                            (start_hue + (end_hue - start_hue) * t) as u16;
                                        updates.insert(
                                            i,
                                            HSBK {
                                                hue: zone_hue,
                                                saturation: after_color.next.saturation,
                                                brightness: after_color.next.brightness,
                                                kelvin: after_color.next.kelvin,
                                            },
                                        );
                                    }
                                    if let Err(e) = self.lighting_manager.set_extended_color_zones(
                                        &&**bulb, zones, &updates, duration,
                                    ) {
                                        log::error!("Error setting matrix gradient: {}", e);
                                    } else {
                                        self.success_toast(&t!("controls.gradient_applied"));
                                    }
                                }
                            }
                        }
                    }
                }

                if before_color != after_color.next {
                    match device {
                        DeviceInfo::Bulb(bulb) => {
                            let selected = self
                                .selected_zones
                                .get(&device_id)
                                .cloned()
                                .unwrap_or_default();

                            if bulb.is_matrix() && !selected.is_empty() {
                                let duration = after_color.duration.unwrap_or(0);
                                if let Some(zones) = bulb.get_zone_colors() {
                                    let updates: HashMap<usize, HSBK> = selected
                                        .iter()
                                        .map(|&idx| (idx, after_color.next))
                                        .collect();
                                    if let Err(e) = self.lighting_manager.set_extended_color_zones(
                                        &&**bulb, zones, &updates, duration,
                                    ) {
                                        log::error!("Error setting matrix color: {}", e);
                                        self.error_toast(&t!(
                                            "error.matrix_color",
                                            error = e.to_string()
                                        ));
                                    }
                                }
                            } else if bulb.is_multizone() && !selected.is_empty() {
                                let duration = after_color.duration.unwrap_or(0);
                                let ranges = contiguous_ranges(&selected);
                                for (i, (start, end)) in ranges.iter().enumerate() {
                                    let apply = if i == ranges.len() - 1 {
                                        ApplicationRequest::Apply
                                    } else {
                                        ApplicationRequest::NoApply
                                    };
                                    if let Err(e) = self.lighting_manager.set_color_zones(
                                        &&**bulb,
                                        *start as u8,
                                        *end as u8,
                                        after_color.next,
                                        duration,
                                        apply,
                                    ) {
                                        log::error!("Error setting zone color: {}", e);
                                        self.error_toast(&t!(
                                            "error.zone_color",
                                            error = e.to_string()
                                        ));
                                        break;
                                    }
                                }
                            } else {
                                if let Err(e) = self.lighting_manager.set_color(
                                    &&**bulb,
                                    after_color.next,
                                    after_color.duration,
                                ) {
                                    log::error!("Error setting color: {}", e);
                                    self.error_toast(&t!("error.set_color", error = e.to_string()));
                                }
                            }
                        }
                        DeviceInfo::Group(group) => {
                            if let Err(e) = self.lighting_manager.set_group_color(
                                group,
                                after_color.next,
                                bulbs,
                                after_color.duration,
                            ) {
                                log::error!("Error setting group color: {}", e);
                                self.error_toast(&t!("error.group_color", error = e.to_string()));
                            }
                        }
                    }
                }
            } else {
                ui.label(
                    t!("devices.no_color_data", data = format!("{:?}", color_opt)).to_string(),
                );
            }
        });
    }

    fn display_device(
        &mut self,
        ui: &mut Ui,
        device: &DeviceInfo,
        bulbs: &mut MutexGuard<HashMap<u64, BulbInfo>>,
    ) {
        ui.add_space(2.0);

        let card_id = ui.make_persistent_id(("device_card", device.id()));
        let prev_hovered: bool = ui.data(|d| d.get_temp(card_id).unwrap_or(false));
        let hover_t = ui.ctx().animate_bool_responsive(card_id, prev_hovered);

        let fill = Color32::from_rgb(
            (30.0 + 8.0 * hover_t) as u8,
            (30.0 + 8.0 * hover_t) as u8,
            (42.0 + 12.0 * hover_t) as u8,
        );
        let stroke_color = Color32::from_rgb(
            (50.0 + 30.0 * hover_t) as u8,
            (50.0 + 30.0 * hover_t) as u8,
            (65.0 + 45.0 * hover_t) as u8,
        );

        let frame_resp = egui::Frame::none()
            .rounding(egui::Rounding::same(10.0))
            .inner_margin(egui::Margin::same(12.0))
            .fill(fill)
            .stroke(Stroke::new(1.0 + 0.5 * hover_t, stroke_color))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                let color = self.get_device_display_color(ui, device, bulbs);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    display_color_circle(
                        ui,
                        device,
                        color.unwrap_or(default_hsbk()),
                        Vec2::new(1.0, 1.0),
                        8.0,
                        bulbs,
                    );
                    self.render_device_controls(ui, device, color, bulbs);
                });
            });

        let is_hovered = frame_resp.response.hovered();
        ui.data_mut(|d| d.insert_temp(card_id, is_hovered));
    }

    fn display_color_controls(
        &mut self,
        ui: &mut Ui,
        device: &DeviceInfo,
        color: HSBK,
    ) -> DeltaColor {
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

        const PRESETS: &[(&str, HSBK)] = &[
            (
                "Warm",
                HSBK {
                    hue: 0,
                    saturation: 0,
                    brightness: 65535,
                    kelvin: 2700,
                },
            ),
            (
                "Day",
                HSBK {
                    hue: 0,
                    saturation: 0,
                    brightness: 65535,
                    kelvin: 5600,
                },
            ),
            (
                "Cool",
                HSBK {
                    hue: 0,
                    saturation: 0,
                    brightness: 65535,
                    kelvin: 9000,
                },
            ),
            (
                "Red",
                HSBK {
                    hue: 0,
                    saturation: 65535,
                    brightness: 65535,
                    kelvin: 3500,
                },
            ),
            (
                "Orange",
                HSBK {
                    hue: 5461,
                    saturation: 65535,
                    brightness: 65535,
                    kelvin: 3500,
                },
            ),
            (
                "Yellow",
                HSBK {
                    hue: 10922,
                    saturation: 65535,
                    brightness: 65535,
                    kelvin: 3500,
                },
            ),
            (
                "Green",
                HSBK {
                    hue: 21845,
                    saturation: 65535,
                    brightness: 65535,
                    kelvin: 3500,
                },
            ),
            (
                "Blue",
                HSBK {
                    hue: 43690,
                    saturation: 65535,
                    brightness: 65535,
                    kelvin: 3500,
                },
            ),
            (
                "Purple",
                HSBK {
                    hue: 54613,
                    saturation: 65535,
                    brightness: 65535,
                    kelvin: 3500,
                },
            ),
        ];

        ui.add_space(2.0);
        let mut remove_custom_idx: Option<usize> = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            color_wheel(ui, &mut hue, &mut saturation, 28.0);
            ui.add_space(2.0);
            for (label, preset) in PRESETS {
                let swatch_color = Color32::from(crate::HSBK32::from(*preset));
                let size = egui::vec2(18.0, 18.0);
                let (resp, painter) = ui.allocate_painter(size, egui::Sense::click());
                let rounding = egui::Rounding::same(3.0);
                painter.rect_filled(resp.rect, rounding, swatch_color);
                if resp.hovered() {
                    painter.rect_stroke(resp.rect, rounding, Stroke::new(1.5, Color32::WHITE));
                }
                if resp.clicked() {
                    hue = preset.hue;
                    saturation = preset.saturation;
                    brightness = preset.brightness;
                    kelvin = preset.kelvin;
                }
                resp.on_hover_text(*label);
            }

            for (idx, (label, custom)) in self.settings.custom_colors.iter().enumerate() {
                let preset_hsbk = HSBK::from(*custom);
                let swatch_color = Color32::from(crate::HSBK32::from(preset_hsbk));
                let size = egui::vec2(18.0, 18.0);
                let (resp, painter) = ui.allocate_painter(size, egui::Sense::click());
                let rounding = egui::Rounding::same(3.0);
                painter.rect_filled(resp.rect, rounding, swatch_color);
                painter.rect_stroke(
                    resp.rect.shrink(1.0),
                    egui::Rounding::same(2.0),
                    Stroke::new(0.5, Color32::from_white_alpha(60)),
                );
                if resp.hovered() {
                    painter.rect_stroke(resp.rect, rounding, Stroke::new(1.5, Color32::WHITE));
                }
                if resp.clicked() {
                    hue = preset_hsbk.hue;
                    saturation = preset_hsbk.saturation;
                    brightness = preset_hsbk.brightness;
                    kelvin = preset_hsbk.kelvin;
                }
                resp.context_menu(|ui| {
                    if ui.button(t!("preset.remove_custom")).clicked() {
                        remove_custom_idx = Some(idx);
                        ui.close_menu();
                    }
                });
                resp.on_hover_text(label.as_str());
            }

            let size = egui::vec2(18.0, 18.0);
            let (resp, painter) = ui.allocate_painter(size, egui::Sense::click());
            let rounding = egui::Rounding::same(3.0);
            let plus_fill = if resp.hovered() {
                Color32::from_rgb(55, 55, 75)
            } else {
                Color32::from_rgb(40, 40, 55)
            };
            painter.rect_filled(resp.rect, rounding, plus_fill);
            painter.text(
                resp.rect.center(),
                Align2::CENTER_CENTER,
                "+",
                egui::FontId::proportional(14.0),
                Color32::from_white_alpha(180),
            );
            if resp.clicked() {
                let color = crate::HSBK32 {
                    hue: hue as u32,
                    saturation: saturation as u32,
                    brightness: brightness as u32,
                    kelvin: kelvin as u32,
                };
                let name = format!(
                    "{} #{}",
                    t!("preset.custom_prefix"),
                    self.settings.custom_colors.len() + 1,
                );
                self.settings.custom_colors.push((name, color));
                self.info_toast(&t!("preset.custom_added"));
            }
            resp.on_hover_text(t!("preset.add_custom"));
        });
        if let Some(idx) = remove_custom_idx {
            self.settings.custom_colors.remove(idx);
            self.info_toast(&t!("preset.custom_removed"));
        }

        ui.add_space(4.0);
        rgb_input(
            ui,
            &mut hue,
            &mut saturation,
            &mut brightness,
            &mut kelvin,
            color,
        );

        let duration = if self.settings.transition_duration_ms > 0 {
            Some(self.settings.transition_duration_ms as u32)
        } else {
            None
        };
        DeltaColor {
            next: HSBK {
                hue,
                saturation,
                brightness,
                kelvin,
            },
            duration,
        }
    }

    fn file_menu_button(&mut self, ui: &mut Ui) {
        let close_shortcut = egui::KeyboardShortcut::new(Modifiers::CTRL, egui::Key::Q);
        let refresh_shortcut = egui::KeyboardShortcut::new(Modifiers::NONE, egui::Key::F5);
        if ui.input_mut(|i| i.consume_shortcut(&close_shortcut)) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if ui.input_mut(|i| i.consume_shortcut(&refresh_shortcut)) {
            if let Err(e) = self.lighting_manager.refresh() {
                log::error!("Error refreshing manager: {}", e);
                self.error_toast(&t!("error.refresh", error = e.to_string()));
            }
        }

        ui.menu_button(t!("menu.file").to_string(), |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            if ui
                .add(
                    egui::Button::new(t!("menu.refresh").to_string())
                        .shortcut_text(ui.ctx().format_shortcut(&refresh_shortcut)),
                )
                .on_hover_text(t!("menu.refresh_hover").to_string())
                .clicked()
            {
                if let Err(e) = self.lighting_manager.refresh() {
                    log::error!("Error refreshing manager: {}", e);
                    self.error_toast(&t!("error.refresh", error = e.to_string()));
                }
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new(t!("menu.settings").to_string()))
                .on_hover_text(t!("menu.settings_hover").to_string())
                .clicked()
            {
                self.show_settings = true;
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new(t!("menu.audio_debug").to_string()))
                .on_hover_text(t!("menu.audio_debug_hover").to_string())
                .clicked()
            {
                self.show_audio_debug = !self.show_audio_debug;
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new(t!("menu.hide_to_tray").to_string()))
                .on_hover_text(t!("menu.hide_to_tray_hover").to_string())
                .clicked()
            {
                self.window_visible.store(false, Ordering::SeqCst);
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                ui.close_menu();
            }
            if ui
                .add(
                    egui::Button::new(t!("menu.quit").to_string())
                        .shortcut_text(ui.ctx().format_shortcut(&close_shortcut)),
                )
                .on_hover_text(t!("menu.quit_hover").to_string())
                .clicked()
            {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                ui.close_menu();
            }
        });
    }

    fn help_menu_button(&mut self, ui: &mut Ui) {
        ui.menu_button(t!("menu.help").to_string(), |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            if ui
                .add(egui::Button::new(t!("menu.about").to_string()))
                .on_hover_text(t!("menu.about_hover").to_string())
                .clicked()
            {
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
                ui.separator();
                let search_field = ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .desired_width(120.0)
                        .hint_text(t!("devices.search_placeholder").to_string()),
                );
                if ui.input_mut(|i| {
                    i.consume_shortcut(&egui::KeyboardShortcut::new(Modifiers::CTRL, egui::Key::F))
                }) {
                    search_field.request_focus();
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.search_query.is_empty() {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(
                            t!("devices.filtering", query = &self.search_query).to_string(),
                        )
                        .size(11.0)
                        .color(Color32::from_rgb(180, 120, 30)),
                    );
                    if ui.small_button(t!("devices.clear").to_string()).clicked() {
                        self.search_query.clear();
                    }
                });
                ui.add_space(2.0);
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                let bulbs = self.lighting_manager.bulbs.clone();
                let bulbs = bulbs.lock();
                ui.vertical(|ui| {
                    if let Ok(mut bulbs) = bulbs {
                        if bulbs.is_empty() {
                            ui.add_space(40.0);
                            ui.vertical_centered(|ui| {
                                ui.add(egui::Spinner::new().size(32.0));
                                ui.add_space(12.0);
                                ui.label(
                                    RichText::new(t!("devices.searching").to_string())
                                        .size(16.0)
                                        .color(Color32::from_rgb(160, 160, 180)),
                                );
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new(t!("devices.searching_hint").to_string())
                                        .size(12.0)
                                        .color(Color32::from_rgb(120, 120, 140)),
                                );
                                ui.add_space(12.0);
                                if ui.button(t!("devices.refresh").to_string()).clicked() {
                                    if let Err(e) = self.lighting_manager.discover() {
                                        log::error!("Failed to discover bulbs: {}", e);
                                        self.error_toast(&t!(
                                            "error.discover",
                                            error = e.to_string()
                                        ));
                                    }
                                }
                            });
                        } else {
                            self.display_device(
                                ui,
                                &DeviceInfo::Group(self.lighting_manager.all_bulbs_group.clone()),
                                &mut bulbs,
                            );
                            let (grouped, ungrouped) = {
                                let sorted_bulbs = self.sort_bulbs(bulbs.values().collect());
                                let query_lower = self.search_query.to_lowercase();
                                let filtered_bulbs: Vec<&BulbInfo> = sorted_bulbs
                                    .into_iter()
                                    .filter(|bulb| {
                                        self.search_query.is_empty()
                                            || bulb
                                                .name_label()
                                                .map(|n| n.to_lowercase().contains(&query_lower))
                                                .unwrap_or(false)
                                            || bulb
                                                .group_label()
                                                .map(|g| g.to_lowercase().contains(&query_lower))
                                                .unwrap_or(false)
                                    })
                                    .collect();

                                let mut grouped: Vec<(crate::device_info::GroupInfo, Vec<u64>)> =
                                    Vec::new();
                                let mut ungrouped: Vec<u64> = Vec::new();

                                for bulb in &filtered_bulbs {
                                    if let Some(group) = bulb.group.data.as_ref() {
                                        let group_name =
                                            group.label.cstr().to_str().unwrap_or_default();
                                        if let Some(entry) = grouped.iter_mut().find(|(g, _)| {
                                            g.label.cstr().to_str().unwrap_or_default()
                                                == group_name
                                        }) {
                                            entry.1.push(bulb.target);
                                        } else {
                                            grouped.push((group.clone(), vec![bulb.target]));
                                        }
                                    } else {
                                        ungrouped.push(bulb.target);
                                    }
                                }
                                (grouped, ungrouped)
                            };

                            for (group, target_ids) in &grouped {
                                let group_id =
                                    ui.make_persistent_id(("group_collapse", group.id()));
                                egui::collapsing_header::CollapsingState::load_with_default_open(
                                    ui.ctx(),
                                    group_id,
                                    true,
                                )
                                .show_header(ui, |ui| {
                                    self.display_device(
                                        ui,
                                        &DeviceInfo::Group(group.clone()),
                                        &mut bulbs,
                                    );
                                })
                                .body(|ui| {
                                    for target in target_ids {
                                        if let Some(bulb) = bulbs.get(target) {
                                            let bulb = bulb.clone();
                                            self.display_device(
                                                ui,
                                                &DeviceInfo::Bulb(Box::new(bulb)),
                                                &mut bulbs,
                                            );
                                        }
                                    }
                                });
                            }
                            for target in &ungrouped {
                                if let Some(bulb) = bulbs.get(target) {
                                    let bulb = bulb.clone();
                                    self.display_device(
                                        ui,
                                        &DeviceInfo::Bulb(Box::new(bulb)),
                                        &mut bulbs,
                                    );
                                }
                            }
                        }
                    }
                });
            });
        });
    }

    fn show_about_window(&mut self, ctx: &egui::Context) {
        if self.show_about {
            egui::Window::new(t!("about.title").to_string())
                .default_width(ABOUT_WINDOW_SIZE[0])
                .default_height(ABOUT_WINDOW_SIZE[1])
                .open(&mut self.show_about)
                .resizable([true, false])
                .show(ctx, |ui| {
                    ui.heading(capitalize_first_letter(env!("CARGO_PKG_NAME")));
                    ui.add_space(8.0);
                    ui.label(env!("CARGO_PKG_DESCRIPTION"));
                    ui.label(t!("about.version", version = env!("CARGO_PKG_VERSION")).to_string());
                    ui.label(t!("about.author", author = env!("CARGO_PKG_AUTHORS")).to_string());
                    ui.hyperlink_to(t!("about.github").to_string(), env!("CARGO_PKG_REPOSITORY"));
                });
        }
    }

    fn show_audio_debug_window(&mut self, ctx: &egui::Context) {
        if self.show_audio_debug {
            egui::Window::new(t!("audio_debug.title").to_string())
                .default_width(ABOUT_WINDOW_SIZE[0])
                .default_height(ABOUT_WINDOW_SIZE[1])
                .open(&mut self.show_audio_debug)
                .resizable([true, false])
                .show(ctx, |ui| {
                    ui.heading(t!("audio_debug.title").to_string());
                    ui.add_space(8.0);
                    self.audio_manager.ui(ui);
                });
        }
    }

    fn check_scheduled_scenes(&mut self) {
        use std::time::SystemTime;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let secs_today = now % 86400;
        let today_date = (now / 86400) as u32;

        let to_fire: Vec<usize> = self
            .settings
            .scheduled_scenes
            .iter()
            .enumerate()
            .filter(|(_, sched)| {
                if !sched.enabled {
                    return false;
                }
                let target_secs = sched.hour as u64 * 3600 + sched.minute as u64 * 60;
                let already_fired = sched
                    .last_fired_date
                    .map(|(d, _, _)| d == today_date)
                    .unwrap_or(false);
                secs_today >= target_secs && secs_today < target_secs + 60 && !already_fired
            })
            .map(|(i, _)| i)
            .collect();

        for i in to_fire {
            let scene_name = self.settings.scheduled_scenes[i].scene_name.clone();
            if let Some(scene) = self.settings.scenes.iter().find(|s| s.name == scene_name) {
                let scene = scene.clone();
                if let Err(e) = scene.apply(&mut self.lighting_manager) {
                    log::error!("Scheduled scene '{}' failed: {:?}", scene_name, e);
                } else {
                    log::info!("Scheduled scene '{}' applied", scene_name);
                }
                self.settings.scheduled_scenes[i].last_fired_date = Some((today_date, 0, 0));
            }
        }
    }

    fn show_toasts(&mut self, ctx: &egui::Context) {
        self.toasts.show(ctx);
    }

    fn toast_template(&mut self, text: &str, kind: ToastKind) -> Toast {
        Toast {
            text: text.into(),
            kind,
            options: ToastOptions::default()
                .duration_in_seconds(3.0)
                .show_progress(true),
            ..Default::default()
        }
    }

    pub fn success_toast(&mut self, text: &str) {
        let toast = self.toast_template(text, ToastKind::Success);
        self.toasts.add(toast);
    }

    pub fn error_toast(&mut self, text: &str) {
        let toast = self.toast_template(text, ToastKind::Error);
        self.toasts.add(toast);
    }

    pub fn info_toast(&mut self, text: &str) {
        let toast = self.toast_template(text, ToastKind::Info);
        self.toasts.add(toast);
    }

    pub fn warning_toast(&mut self, text: &str) {
        let toast = self.toast_template(text, ToastKind::Warning);
        self.toasts.add(toast);
    }
}

impl eframe::App for MantleApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(feature = "puffin")]
        puffin::GlobalProfiler::lock().new_frame();

        self.handle_tray_events(ctx);
        ctx.request_repaint_after(Duration::from_millis(self.settings.refresh_rate_ms));

        if Instant::now() - self.lighting_manager.last_discovery
            > Duration::from_millis(self.settings.refresh_rate_ms)
        {
            if let Err(e) = self.lighting_manager.discover() {
                log::error!("Failed to discover bulbs: {}", e);
            }
        }
        let poll_interval = Duration::from_millis(self.settings.refresh_rate_ms);
        if self.last_refresh.elapsed() >= poll_interval {
            if let Err(e) = self.lighting_manager.refresh() {
                log::error!("Error refreshing manager: {}", e);
                self.error_toast(&t!("error.refresh", error = e.to_string()));
            }
            self.last_refresh = Instant::now();
        }
        if self.last_schedule_check.elapsed() >= Duration::from_secs(1) {
            self.check_scheduled_scenes();
            self.last_schedule_check = Instant::now();
        }

        if !self.window_visible.load(Ordering::SeqCst) {
            ctx.request_repaint_after(Duration::from_secs(2));
            return;
        }

        self.update_ui(ctx);
        self.show_about_window(ctx);
        self.show_audio_debug_window(ctx);
        self.settings_ui(ctx);
        self.show_toasts(ctx);
    }
}

/// Collapse a set of zone indices into sorted contiguous (start, end) ranges.
fn contiguous_ranges(zones: &HashSet<usize>) -> Vec<(usize, usize)> {
    if zones.is_empty() {
        return Vec::new();
    }
    let mut sorted: Vec<usize> = zones.iter().copied().collect();
    sorted.sort_unstable();
    let mut ranges = Vec::new();
    let mut start = sorted[0];
    let mut end = start;
    for &z in &sorted[1..] {
        if z == end + 1 {
            end = z;
        } else {
            ranges.push((start, end));
            start = z;
            end = z;
        }
    }
    ranges.push((start, end));
    ranges
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(target_os = "linux")]
    fn tray_menu_creation_requires_gtk_init() {
        gtk::init().expect("Failed to initialize GTK");
        let menu = tray_icon::menu::Menu::new();
        let _item = tray_icon::menu::MenuItem::new("Test", true, None);
        let _ = menu.append(&_item);
    }
}
