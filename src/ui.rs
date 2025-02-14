use std::{
    collections::HashMap,
    ops::RangeInclusive,
    sync::{mpsc, Arc, Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

use crate::{
    app::{
        ColorChannelEntry, MantleApp, WaveformTracker, AUDIO_ICON, EYEDROPPER_ICON, ICON,
        MAIN_WINDOW_SIZE, MIN_WINDOW_SIZE, MONITOR_ICON, SUBREGION_ICON,
    },
    audio::AudioManager,
    color::{kelvin_to_rgb, DeltaColor},
    contrast_color,
    device_info::DeviceInfo,
    products::{KELVIN_RANGE, LIFX_RANGE},
    screencap::{RegionCaptureTarget, ScreenSubregion, ScreencapManager},
    AngleIter, BulbInfo, LifxManager, HSBK32, RGB8,
};

use eframe::{
    egui::{
        self, lerp, pos2, remap_clamp, vec2, Color32, Mesh, Pos2, Response, Sense, Shape, Stroke,
        Ui, Vec2, WidgetInfo, WidgetType,
    },
    epaint::CubicBezierShape,
};
use image::GenericImageView;
use lifx_core::HSBK;

const DEBOUNCE_DELAY_MS: u64 = 100;
const SLIDER_RESOLUTION: u32 = 36;

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

pub fn create_highlighted_button(
    ui: &mut Ui,
    icon_name: &'static str,
    icon: &[u8],
    active: bool,
) -> Response {
    let highlight = if active {
        ui.visuals().widgets.hovered.bg_stroke.color
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };
    ui.add(
        egui::Button::image(
            egui::Image::from_bytes(icon_name, icon.to_vec())
                .fit_to_exact_size(ui.spacing().interact_size),
        )
        .sense(Sense::click())
        .fill(highlight),
    )
}

pub fn handle_eyedropper(
    app: &mut MantleApp,
    ui: &mut Ui,
    device: &DeviceInfo,
) -> Option<DeltaColor> {
    let mut color: Option<HSBK> = None;
    let show_eyedropper = app.show_eyedropper.entry(device.id()).or_insert(false);
    if create_highlighted_button(ui, "eyedropper", EYEDROPPER_ICON, *show_eyedropper).clicked() {
        *show_eyedropper = !*show_eyedropper;
    }
    if *show_eyedropper {
        let screencap = ScreencapManager::new().expect("Failed to create screencap manager");
        ui.ctx().output_mut(|out| {
            out.cursor_icon = egui::CursorIcon::Crosshair;
        });
        if app.input_listener.is_button_pressed(rdev::Button::Left) {
            let position = app
                .input_listener
                .get_last_mouse_position()
                .expect("Failed to get mouse position");
            match screencap.color_from_click(position.x, position.y) {
                Ok(c) => color = Some(c),
                Err(e) => eprintln!("Failed to get color: {}", e),
            }
            *show_eyedropper = false;
        }
    }
    color.map(|color| DeltaColor {
        next: color,
        duration: None,
    })
}

pub fn handle_screencap(
    app: &mut MantleApp,
    ui: &mut Ui,
    device: &DeviceInfo,
) -> Option<DeltaColor> {
    let mut color: Option<HSBK> = None;
    let update_interval_ms = app.settings.update_interval_ms;
    update_subregion_bounds(app, ui, device.id());
    if let Some(color_channel) = app.waveform_channel.get(&device.id()) {
        let tracker: &mut WaveformTracker =
            app.waveform_map
                .entry(device.id())
                .or_insert(WaveformTracker {
                    active: false,
                    last_update: Instant::now(),
                    region: RegionCaptureTarget::All,
                    stop_tx: None,
                });
        if tracker.active && (has_time_elapsed(update_interval_ms, tracker)) {
            update_color_from_channel(&mut color, tracker, color_channel);
        }
    } else {
        let (tx, rx) = mpsc::channel();
        app.waveform_channel.insert(
            device.id(),
            ColorChannelEntry {
                tx,
                rx,
                handle: None,
            },
        );
    }

    let is_active = app.waveform_map.get(&device.id()).is_some_and(|w| w.active);
    if create_highlighted_button(ui, "monitor", MONITOR_ICON, is_active).clicked() {
        initialize_waveform_tracker(app, device, update_interval_ms);
    }
    if let Some(waveform) = app.waveform_map.get_mut(&device.id()) {
        ui.vertical(|ui| {
            let mut subregion = app
                .subregion_points
                .entry(device.id())
                .or_default()
                .lock()
                .expect("Failed to get subregion");

            // Create options for ComboBox with consistent ordering
            let mut options = vec![("All".to_string(), RegionCaptureTarget::All)];

            // Collect monitor options
            let mut monitor_options: Vec<(String, RegionCaptureTarget)> = app
                .screen_manager
                .monitors
                .iter()
                .map(|monitor| {
                    (
                        monitor.name().to_string(),
                        RegionCaptureTarget::Monitor(vec![monitor.clone()]),
                    )
                })
                .collect();
            // Sort monitor options to ensure consistent order
            monitor_options.sort_by(|a, b| a.0.cmp(&b.0));
            options.extend(monitor_options);

            // Collect window options
            let mut window_options: Vec<(String, RegionCaptureTarget)> = app
                .screen_manager
                .windows
                .iter()
                .map(|window| {
                    (
                        window.title().to_string(),
                        RegionCaptureTarget::Window(vec![window.clone()]),
                    )
                })
                .collect();
            // Sort window options to ensure consistent order
            window_options.sort_by(|a, b| a.0.cmp(&b.0));
            options.extend(window_options);

            options.push((
                "Subregion".to_string(),
                RegionCaptureTarget::Subregion(vec![subregion.clone()]),
            ));

            // Determine the selected text
            let selected_text = match &waveform.region {
                RegionCaptureTarget::All => "All".to_string(),
                RegionCaptureTarget::Monitor(monitors) => monitors
                    .first()
                    .map(|m| m.name().to_string())
                    .unwrap_or("Monitor".to_string()),
                RegionCaptureTarget::Window(windows) => windows
                    .first()
                    .map(|w| w.title().to_string())
                    .unwrap_or("Window".to_string()),
                RegionCaptureTarget::Subregion(_) => "Subregion".to_string(),
            };
            // Use ComboBox with consistent ID
            ui.push_id(device.id(), |ui| {
                egui::ComboBox::from_label("Capture Type")
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        for (label, capture_target) in options {
                            ui.selectable_value(&mut waveform.region, capture_target, label);
                        }
                    });
            });

            // If the selected target is Subregion, display the numerical fields
            if let RegionCaptureTarget::Subregion(_) = waveform.region {
                ui.horizontal(|ui| {
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut subregion.x));

                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut subregion.y));

                    ui.label("Width:");
                    ui.add(egui::DragValue::new(&mut subregion.width));

                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut subregion.height));
                });
            }
        });
    }

    color.map(|color| DeltaColor {
        next: color,
        duration: Some((update_interval_ms / 2) as u32),
    })
}

pub fn update_subregion_bounds(app: &mut MantleApp, ui: &mut Ui, device_id: u64) {
    // Get or create the subregion
    let subregion_lock = app
        .subregion_points
        .entry(device_id)
        .or_insert_with(|| Arc::new(Mutex::new(ScreenSubregion::default())));
    let show_subregion = app.show_subregion.entry(device_id).or_insert(false);

    let mut subregion = subregion_lock.lock().expect("Failed to get subregion");

    if create_highlighted_button(ui, "subregion", SUBREGION_ICON, *show_subregion).clicked() {
        *show_subregion = !*show_subregion;
        if *show_subregion {
            subregion.reset();
        }
    }
    if *show_subregion {
        if app.input_listener.is_button_pressed(rdev::Button::Left) {
            let mouse_pos = app
                .input_listener
                .get_last_mouse_position()
                .expect("Failed to get mouse position");
            if subregion.x == 0 && subregion.y == 0 {
                subregion.x = mouse_pos.x;
                subregion.y = mouse_pos.y;
                // debounce the click (dirty but only way I can think of)
                thread::sleep(Duration::from_millis(DEBOUNCE_DELAY_MS));
            } else {
                subregion.width = (mouse_pos.x - subregion.x).unsigned_abs();
                subregion.height = (mouse_pos.y - subregion.y).unsigned_abs();
                *show_subregion = false;
            }
        } else if app.input_listener.is_key_pressed(rdev::Key::Escape) {
            *show_subregion = false;
        }
    }
}

pub fn display_color_circle(
    ui: &mut Ui,
    device: &DeviceInfo,
    color: HSBK,
    desired_size: Vec2,
    scale: f32,
    bulbs: &MutexGuard<HashMap<u64, BulbInfo>>,
) {
    let power = match device {
        DeviceInfo::Bulb(bulb) => bulb.power_level.data.unwrap_or(0),
        DeviceInfo::Group(group) => group.is_any_bulb_on(bulbs) as u16 * u16::MAX,
    };
    let desired_size = ui.spacing().interact_size * desired_size;
    // Arc code from https://vcs.cozydsp.space/cozy-dsp/cozy-ui/src/commit/d4706ec9f4592137307ce8acafb56b881ea54e35/src/util.rs#L49
    let rgb = RGB8::from(color);
    let (response, painter) = ui.allocate_painter(desired_size, Sense::hover());
    let center = response.rect.center();
    let radius = response.rect.width() / scale;
    let inner_stroke = Stroke::new(radius / 2.0, Color32::from(rgb));
    let outer_stroke = Stroke::new((5. / 6.) * radius, Color32::from_gray(64));
    let off_stroke = Stroke::new((5. / 6.) * radius, Color32::from_gray(32));
    let start_angle: f32 = 0.0;
    let end_angle: f32 = (2.0 * std::f32::consts::PI) * (color.brightness as f32 / u16::MAX as f32);
    if power != 0.0 as u16 {
        painter.circle(center, radius, Color32::TRANSPARENT, outer_stroke);
        painter.extend(
            AngleIter::new(start_angle, end_angle).map(|(start_angle, end_angle)| {
                let xc = center.x;
                let yc = center.y;
                let p1 = center + radius * Vec2::new(start_angle.cos(), -start_angle.sin());
                let p4 = center + radius * Vec2::new(end_angle.cos(), -end_angle.sin());
                let a = p1 - center;
                let b = p4 - center;
                let q1 = a.length_sq();
                let q2 = q1 + a.dot(b);
                let k2 = (4.0 / 3.0) * ((2.0 * q1 * q2).sqrt() - q2) / (a.x * b.y - a.y * b.x);

                let p2 = Pos2::new(xc + a.x - k2 * a.y, yc + a.y + k2 * a.x);
                let p3 = Pos2::new(xc + b.x + k2 * b.y, yc + b.y - k2 * b.x);

                Shape::CubicBezier(CubicBezierShape::from_points_stroke(
                    [p1, p2, p3, p4],
                    false,
                    Color32::TRANSPARENT,
                    inner_stroke,
                ))
            }),
        );
    } else {
        painter.circle(center, radius, Color32::TRANSPARENT, off_stroke);
    }
}

pub fn toggle_button(
    ui: &mut Ui,
    lifx_manager: &LifxManager,
    device: &DeviceInfo,
    scale: Vec2,
    registered_bulbs: &MutexGuard<HashMap<u64, BulbInfo>>,
) -> egui::Response {
    let desired_size = ui.spacing().interact_size * scale;
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());
    ui.horizontal(|ui| {
        let on = match device {
            DeviceInfo::Bulb(bulb) => bulb.power_level.data.unwrap_or(0) != 0,
            DeviceInfo::Group(group) => group.is_any_bulb_on(registered_bulbs),
        };
        if response.clicked() {
            let level = if on { 0 } else { u16::MAX };
            match device {
                DeviceInfo::Bulb(bulb) => {
                    if let Err(e) = lifx_manager.set_power(&&**bulb, level) {
                        log::error!("Error toggling bulb: {}", e);
                    } else {
                        log::info!("Toggled bulb {:?}", bulb.name);
                    }
                }
                DeviceInfo::Group(group) => {
                    if let Err(e) = lifx_manager.set_group_power(group, registered_bulbs, level) {
                        log::error!("Error toggling group: {}", e);
                    } else {
                        log::info!("Toggled group {:?}", group.label);
                    }
                }
            }
            response.mark_changed();
        }
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::Checkbox, ui.is_enabled(), on, "Toggle")
        });
        if ui.is_rect_visible(rect) {
            let how_on = ui.ctx().animate_bool_responsive(response.id, on);
            let visuals = ui.style().interact_selectable(&response, on);
            let rect = rect.expand(visuals.expansion);
            let radius = 0.5 * rect.height();
            ui.painter()
                .rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);
            let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
            let center = egui::pos2(circle_x, rect.center().y);
            ui.painter()
                .circle(center, 0.75 * radius, visuals.bg_fill, visuals.fg_stroke);
        }
    });
    response
}

pub fn color_slider(
    ui: &mut Ui,
    value: &mut u16,
    range: std::ops::RangeInclusive<u16>,
    label: &str,
    get_color_at_value: impl Fn(u16) -> Color32,
) -> Response {
    let desired_size = vec2(ui.spacing().slider_width, ui.spacing().interact_size.y);
    let (rect, response) = ui.allocate_at_least(desired_size, Sense::click_and_drag());

    if let Some(mpos) = response.interact_pointer_pos() {
        *value = remap_clamp(
            mpos.x,
            rect.left()..=rect.right(),
            RangeInclusive::new(*range.start() as f32, *range.end() as f32),
        )
        .round() as u16;
    }

    response.widget_info(|| {
        WidgetInfo::selected(
            WidgetType::Slider,
            ui.is_enabled(),
            response.drag_started(),
            label,
        )
    });

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact(&response);

        {
            // fill color:
            let mut mesh = Mesh::default();
            for i in 0..=SLIDER_RESOLUTION {
                let t = i as f32 / (SLIDER_RESOLUTION as f32);
                let color = get_color_at_value((t * u16::MAX as f32) as u16);
                let x = lerp(rect.left()..=rect.right(), t);
                // round edges:
                let y_offset = if i == 0 || i == SLIDER_RESOLUTION {
                    (ui.spacing().slider_rail_height / 2.0) - 2.
                } else {
                    ui.spacing().slider_rail_height / 2.0
                };
                mesh.colored_vertex(pos2(x, rect.center().y + y_offset), color);
                mesh.colored_vertex(pos2(x, rect.center().y - y_offset), color);
                if i < SLIDER_RESOLUTION {
                    mesh.add_triangle(2 * i, 2 * i + 1, 2 * i + 2);
                    mesh.add_triangle(2 * i + 1, 2 * i + 2, 2 * i + 3);
                }
            }
            ui.painter().add(Shape::mesh(mesh));
        }

        ui.painter().rect_stroke(rect, 0.0, visuals.bg_stroke); // outline

        {
            // Show where the slider is at:
            let x = lerp(
                rect.left()..=rect.right(),
                remap_clamp(
                    *value as f32,
                    RangeInclusive::new(*range.start() as f32, *range.end() as f32),
                    0.0..=1.0,
                ),
            );
            let radius = ui.spacing().slider_rail_height / 2.0 + 2.0;
            let picked_color = get_color_at_value(*value);
            ui.painter().circle(
                pos2(x, rect.center().y),
                radius,
                picked_color,
                Stroke::new(visuals.fg_stroke.width, contrast_color(picked_color)),
            );
        }

        let text_field: &mut String = &mut format!("{}", value);
        let text_response = ui.add(egui::TextEdit::singleline(text_field).desired_width(50.0));
        if text_response.changed() {
            if let Ok(v) = text_field.parse::<u16>() {
                *value = v;
            }
        }
    }

    response
}

pub fn hue_slider(ui: &mut Ui, hue: &mut u16) -> egui::Response {
    color_slider(ui, hue, LIFX_RANGE, "Hue", |v| {
        HSBK32 {
            hue: v as u32,
            saturation: u32::MAX,
            brightness: u32::MAX,
            kelvin: 0,
        }
        .into()
    })
}

pub fn saturation_slider(ui: &mut Ui, saturation: &mut u16) -> egui::Response {
    color_slider(ui, saturation, LIFX_RANGE, "Saturation", |v| {
        let color_value = (u16::MAX - v) / u8::MAX as u16;
        Color32::from_gray(color_value as u8)
    })
}

pub fn brightness_slider(ui: &mut Ui, brightness: &mut u16) -> egui::Response {
    color_slider(ui, brightness, LIFX_RANGE, "Brightness", |v| {
        let color_value = v / u8::MAX as u16;
        Color32::from_gray(color_value as u8)
    })
}

pub fn kelvin_slider(ui: &mut Ui, kelvin: &mut u16, device: &DeviceInfo) -> egui::Response {
    match device {
        DeviceInfo::Bulb(bulb) => {
            if let Some(range) = bulb.features.temperature_range.as_ref() {
                if range.min != range.max {
                    color_slider(
                        ui,
                        kelvin,
                        RangeInclusive::new(range.min as u16, range.max as u16),
                        "Kelvin",
                        |v| {
                            let temp: u16 = remap_clamp(
                                v as f32,
                                0.0..=u16::MAX as f32,
                                KELVIN_RANGE.to_range_f32(),
                            ) as u16;
                            kelvin_to_rgb(temp).into()
                        },
                    )
                } else {
                    ui.label(format!("{}K", range.min))
                }
            } else {
                ui.label("Kelvin")
            }
        }
        DeviceInfo::Group(_) => color_slider(
            ui,
            kelvin,
            RangeInclusive::new(KELVIN_RANGE.min as u16, KELVIN_RANGE.max as u16),
            "Kelvin",
            |v| {
                let temp: u16 =
                    remap_clamp(v as f32, 0.0..=u16::MAX as f32, KELVIN_RANGE.to_range_f32())
                        as u16;
                kelvin_to_rgb(temp).into()
            },
        ),
    }
}

pub fn hsbk_sliders(
    ui: &mut Ui,
    hue: &mut u16,
    saturation: &mut u16,
    brightness: &mut u16,
    kelvin: &mut u16,
    device: &DeviceInfo,
) -> egui::Response {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label("Hue");
            hue_slider(ui, hue)
        });
        ui.horizontal(|ui| {
            ui.label("Saturation");
            saturation_slider(ui, saturation)
        });
        ui.horizontal(|ui| {
            ui.label("Brightness");
            brightness_slider(ui, brightness)
        });
        ui.horizontal(|ui| {
            ui.label("Kelvin");
            kelvin_slider(ui, kelvin, device)
        });
    })
    .response
}

pub fn handle_audio(app: &mut MantleApp, ui: &mut Ui, device: &DeviceInfo) -> Option<DeltaColor> {
    // Similar logic to handle_screencap, using waveform_map & waveform_channel
    let mut color: Option<HSBK> = None;
    let update_interval_ms = app.settings.update_interval_ms;
    if let Some(color_channel) = app.waveform_channel.get(&device.id()) {
        let tracker: &mut WaveformTracker =
            app.waveform_map
                .entry(device.id())
                .or_insert(WaveformTracker {
                    active: false,
                    last_update: Instant::now(),
                    region: RegionCaptureTarget::All,
                    stop_tx: None,
                });
        if tracker.active && (has_time_elapsed(update_interval_ms, tracker)) {
            update_color_from_channel(&mut color, tracker, color_channel);
        }
    } else {
        let (tx, rx) = mpsc::channel();
        app.waveform_channel.insert(
            device.id(),
            ColorChannelEntry {
                tx,
                rx,
                handle: None,
            },
        );
    }

    let is_active = app.waveform_map.get(&device.id()).is_some_and(|w| w.active);
    if create_highlighted_button(ui, "audio", AUDIO_ICON, is_active).clicked() {
        initialize_waveform_tracker(app, device, update_interval_ms);
    }
    color.map(|color| DeltaColor {
        next: color,
        duration: Some((update_interval_ms / 2) as u32),
    })
}

fn initialize_waveform_tracker(app: &mut MantleApp, device: &DeviceInfo, update_interval_ms: u64) {
    if let Some(waveform) = app.waveform_map.get_mut(&device.id()) {
        waveform.active = !waveform.active;
    } else {
        let running_waveform = WaveformTracker {
            active: true,
            last_update: Instant::now(),
            region: RegionCaptureTarget::All,
            stop_tx: None,
        };
        app.waveform_map
            .insert(device.id(), running_waveform.clone());
    }
    // if the waveform is active, we need to spawn a thread to get the color
    if app.waveform_map[&device.id()].active {
        let buffer_clone = app.audio_manager.clone_samples_buffer();
        let tx = app
            .waveform_channel
            .get(&device.id())
            .expect("Failed to get color sender for device")
            .tx
            .clone();
        let lifx_manager = app.lighting_manager.clone();
        let device_id = device.id();

        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        if let Some(waveform_trx) = app.waveform_channel.get_mut(&device.id()) {
            waveform_trx.handle = Some(thread::spawn(move || loop {
                let samples = buffer_clone.lock().unwrap().clone();
                let audio_color = AudioManager::samples_to_hsbk(samples);
                if let Err(err) = lifx_manager.set_color_by_id(device_id, audio_color) {
                    eprintln!("Failed to set color: {}", err);
                }
                if let Err(err) = tx.send(audio_color) {
                    eprintln!("Failed to send color data: {}", err);
                }

                thread::sleep(Duration::from_millis(update_interval_ms / 4));
                if stop_rx.try_recv().is_ok() {
                    break;
                }
            }));
        }
        app.waveform_map
            .get_mut(&device.id())
            .expect("Failed to get stop tx for waveform")
            .stop_tx = Some(stop_tx);
    } else if let Some(waveform_trx) = app.waveform_channel.get_mut(&device.id()) {
        if let Some(thread) = waveform_trx.handle.take() {
            // Send a signal to stop the thread
            if let Some(stop_tx) = app
                .waveform_map
                .get_mut(&device.id())
                .expect("Failed to get waveform")
                .stop_tx
                .take()
            {
                stop_tx.send(()).expect("Failed to send stop signal");
            }
            // Wait for the thread to finish
            thread.join().expect("Failed to join thread");
        }
    }
}

fn has_time_elapsed(update_interval_ms: u64, waveform_tracker: &mut WaveformTracker) -> bool {
    Instant::now() - waveform_tracker.last_update > Duration::from_millis(update_interval_ms)
}

fn update_color_from_channel(
    color: &mut Option<HSBK>,
    tracker: &mut WaveformTracker,
    color_channel: &ColorChannelEntry,
) {
    if let Ok(computed_color) = color_channel.rx.try_recv() {
        *color = Some(computed_color);
        tracker.last_update = Instant::now();
    }
}
