use std::{
    collections::HashMap,
    ops::RangeInclusive,
    sync::{mpsc, Arc, Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

use crate::{
    app::{
        ColorChannelEntry, MantleApp, WaveformMode, WaveformTracker, AUDIO_ICON, EYEDROPPER_ICON,
        ICON, MAIN_WINDOW_SIZE, MIN_WINDOW_SIZE, MONITOR_ICON, SUBREGION_ICON,
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

    ensure_waveform_channel(app, device.id());
    app.waveform_map
        .entry(device.id())
        .or_insert(WaveformTracker {
            active: false,
            last_update: Instant::now(),
            mode: WaveformMode::Screencap,
            region: RegionCaptureTarget::All,
            stop_tx: None,
        });

    if let Some(color_channel) = app.waveform_channel.get(&device.id()) {
        if let Some(tracker) = app.waveform_map.get_mut(&device.id()) {
            if tracker.active
                && tracker.mode == WaveformMode::Screencap
                && has_time_elapsed(update_interval_ms, tracker)
            {
                update_color_from_channel(&mut color, tracker, color_channel);
            }
        }
    }

    let is_active = app
        .waveform_map
        .get(&device.id())
        .is_some_and(|w| w.active && w.mode == WaveformMode::Screencap);
    if create_highlighted_button(ui, "monitor", MONITOR_ICON, is_active).clicked() {
        initialize_waveform_tracker(app, device, update_interval_ms, WaveformMode::Screencap);
    }
    let mut region_type_changed = false;
    if let Some(waveform) = app.waveform_map.get_mut(&device.id()) {
        let prev_discriminant = std::mem::discriminant(&waveform.region);
        ui.vertical(|ui| {
            let mut subregion = app
                .subregion_points
                .entry(device.id())
                .or_default()
                .lock()
                .expect("Failed to get subregion");

            let mut options = vec![("All".to_string(), RegionCaptureTarget::All)];

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
            monitor_options.sort_by(|a, b| a.0.cmp(&b.0));
            options.extend(monitor_options);

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
            window_options.sort_by(|a, b| a.0.cmp(&b.0));
            options.extend(window_options);

            options.push((
                "Subregion".to_string(),
                RegionCaptureTarget::Subregion(vec![subregion.clone()]),
            ));

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
            ui.push_id(device.id(), |ui| {
                egui::ComboBox::from_label("Capture Target")
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        for (label, capture_target) in options {
                            ui.selectable_value(&mut waveform.region, capture_target, label);
                        }
                    });
            });

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
                render_subregion_preview(ui, &app.screen_manager, &mut subregion);
            }
        });
        region_type_changed = waveform.active
            && waveform.mode == WaveformMode::Screencap
            && std::mem::discriminant(&waveform.region) != prev_discriminant;
    }
    if region_type_changed {
        initialize_waveform_tracker(app, device, update_interval_ms, WaveformMode::Screencap);
    }

    if is_active {
        ui.ctx()
            .request_repaint_after(Duration::from_millis(update_interval_ms));
    }

    color.map(|color| DeltaColor {
        next: color,
        duration: Some((update_interval_ms / 2) as u32),
    })
}

pub fn update_subregion_bounds(app: &mut MantleApp, ui: &mut Ui, device_id: u64) {
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
        ui.ctx().output_mut(|out| {
            out.cursor_icon = egui::CursorIcon::Crosshair;
        });
        if app.input_listener.is_button_pressed(rdev::Button::Left) {
            let mouse_pos = app
                .input_listener
                .get_last_mouse_position()
                .expect("Failed to get mouse position");
            if subregion.x == 0 && subregion.y == 0 {
                let global_x = mouse_pos.x;
                let global_y = mouse_pos.y;
                let monitor = app.screen_manager.monitors.iter().find(|m| {
                    global_x >= m.x()
                        && global_x < m.x() + m.width() as i32
                        && global_y >= m.y()
                        && global_y < m.y() + m.height() as i32
                });
                if let Some(mon) = monitor {
                    subregion.monitor = Some(Arc::new(mon.clone()));
                    subregion.x = global_x - mon.x();
                    subregion.y = global_y - mon.y();
                } else {
                    subregion.x = global_x;
                    subregion.y = global_y;
                }
                thread::sleep(Duration::from_millis(DEBOUNCE_DELAY_MS));
            } else {
                let rel_x = if let Some(ref mon) = subregion.monitor {
                    mouse_pos.x - mon.x()
                } else {
                    mouse_pos.x
                };
                let rel_y = if let Some(ref mon) = subregion.monitor {
                    mouse_pos.y - mon.y()
                } else {
                    mouse_pos.y
                };
                subregion.width = (rel_x - subregion.x).unsigned_abs();
                subregion.height = (rel_y - subregion.y).unsigned_abs();
                *show_subregion = false;
            }
        } else if app.input_listener.is_key_pressed(rdev::Key::Escape) {
            *show_subregion = false;
        }
    }
}

fn render_subregion_preview(
    ui: &mut Ui,
    screen_manager: &ScreencapManager,
    subregion: &mut ScreenSubregion,
) {
    let monitors = &screen_manager.monitors;
    if monitors.is_empty() {
        return;
    }

    let mut x_min = i32::MAX;
    let mut y_min = i32::MAX;
    let mut x_max = i32::MIN;
    let mut y_max = i32::MIN;
    for monitor in monitors {
        x_min = x_min.min(monitor.x());
        y_min = y_min.min(monitor.y());
        x_max = x_max.max(monitor.x() + monitor.width() as i32);
        y_max = y_max.max(monitor.y() + monitor.height() as i32);
    }

    let total_width = (x_max - x_min) as f32;
    let total_height = (y_max - y_min) as f32;
    if total_width <= 0.0 || total_height <= 0.0 {
        return;
    }

    let preview_width = ui.available_width().min(300.0);
    let scale = preview_width / total_width;
    let preview_height = total_height * scale;

    ui.add_space(4.0);
    let (response, painter) =
        ui.allocate_painter(vec2(preview_width, preview_height), Sense::click_and_drag());
    let origin = response.rect.min;

    let preview_to_global = |pos: Pos2| -> (i32, i32) {
        (
            ((pos.x - origin.x) / scale) as i32 + x_min,
            ((pos.y - origin.y) / scale) as i32 + y_min,
        )
    };

    painter.rect_filled(response.rect, 2.0, Color32::from_gray(20));

    for monitor in monitors {
        let mon_rect = egui::Rect::from_min_size(
            pos2(
                origin.x + (monitor.x() - x_min) as f32 * scale,
                origin.y + (monitor.y() - y_min) as f32 * scale,
            ),
            vec2(
                monitor.width() as f32 * scale,
                monitor.height() as f32 * scale,
            ),
        );
        painter.rect(
            mon_rect,
            2.0,
            Color32::from_gray(40),
            Stroke::new(1.0, Color32::from_gray(90)),
        );
        painter.text(
            mon_rect.center(),
            egui::Align2::CENTER_CENTER,
            monitor.name(),
            egui::FontId::proportional(10.0),
            Color32::from_gray(110),
        );
    }

    let drag_start_id = response.id.with("drag_start");

    if response.drag_started() {
        if let Some(pos) = response.interact_pointer_pos() {
            let (gx, gy) = preview_to_global(pos);
            let monitor = monitors.iter().find(|m| {
                gx >= m.x()
                    && gx < m.x() + m.width() as i32
                    && gy >= m.y()
                    && gy < m.y() + m.height() as i32
            });
            if let Some(mon) = monitor {
                let rel_x = (gx - mon.x()).max(0);
                let rel_y = (gy - mon.y()).max(0);
                ui.memory_mut(|mem| {
                    mem.data.insert_temp(drag_start_id, (rel_x, rel_y));
                });
                subregion.monitor = Some(Arc::new(mon.clone()));
                subregion.x = rel_x;
                subregion.y = rel_y;
                subregion.width = 0;
                subregion.height = 0;
            }
        }
    }

    if response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let (gx, gy) = preview_to_global(pos);
            if let Some(start) = ui.memory(|mem| mem.data.get_temp::<(i32, i32)>(drag_start_id)) {
                if let Some(ref mon) = subregion.monitor {
                    let end_x = (gx - mon.x()).clamp(0, mon.width() as i32 - 1);
                    let end_y = (gy - mon.y()).clamp(0, mon.height() as i32 - 1);
                    subregion.x = start.0.min(end_x);
                    subregion.y = start.1.min(end_y);
                    subregion.width = (start.0 - end_x).unsigned_abs();
                    subregion.height = (start.1 - end_y).unsigned_abs();
                }
            }
        }
    }

    if subregion.width > 0 && subregion.height > 0 {
        let (sub_gx, sub_gy) = match subregion.monitor {
            Some(ref mon) => (mon.x() + subregion.x, mon.y() + subregion.y),
            None => (subregion.x, subregion.y),
        };
        let sub_rect = egui::Rect::from_min_size(
            pos2(
                origin.x + (sub_gx - x_min) as f32 * scale,
                origin.y + (sub_gy - y_min) as f32 * scale,
            ),
            vec2(
                subregion.width as f32 * scale,
                subregion.height as f32 * scale,
            ),
        );
        painter.rect(
            sub_rect,
            0.0,
            Color32::from_rgba_unmultiplied(60, 140, 255, 35),
            Stroke::new(2.0, Color32::from_rgb(60, 140, 255)),
        );
    }

    if response.hovered() {
        ui.ctx().output_mut(|out| {
            out.cursor_icon = egui::CursorIcon::Crosshair;
        });
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
    let mut color: Option<HSBK> = None;
    let update_interval_ms = app.settings.update_interval_ms;

    ensure_waveform_channel(app, device.id());
    app.waveform_map
        .entry(device.id())
        .or_insert(WaveformTracker {
            active: false,
            last_update: Instant::now(),
            mode: WaveformMode::Audio,
            region: RegionCaptureTarget::All,
            stop_tx: None,
        });

    if let Some(color_channel) = app.waveform_channel.get(&device.id()) {
        if let Some(tracker) = app.waveform_map.get_mut(&device.id()) {
            if tracker.active
                && tracker.mode == WaveformMode::Audio
                && has_time_elapsed(update_interval_ms, tracker)
            {
                update_color_from_channel(&mut color, tracker, color_channel);
            }
        }
    }

    let is_active = app
        .waveform_map
        .get(&device.id())
        .is_some_and(|w| w.active && w.mode == WaveformMode::Audio);
    if create_highlighted_button(ui, "audio", AUDIO_ICON, is_active).clicked() {
        initialize_waveform_tracker(app, device, update_interval_ms, WaveformMode::Audio);
    }

    if is_active {
        ui.ctx()
            .request_repaint_after(Duration::from_millis(update_interval_ms));
    }

    color.map(|color| DeltaColor {
        next: color,
        duration: Some((update_interval_ms / 2) as u32),
    })
}

fn ensure_waveform_channel(app: &mut MantleApp, device_id: u64) {
    app.waveform_channel.entry(device_id).or_insert_with(|| {
        let (tx, rx) = mpsc::channel();
        ColorChannelEntry {
            tx,
            rx,
            handle: None,
        }
    });
}

fn stop_active_waveform(app: &mut MantleApp, device_id: u64) {
    if let Some(tracker) = app.waveform_map.get_mut(&device_id) {
        if let Some(stop_tx) = tracker.stop_tx.take() {
            let _ = stop_tx.send(());
        }
        tracker.active = false;
    }
    if let Some(channel) = app.waveform_channel.get_mut(&device_id) {
        if let Some(handle) = channel.handle.take() {
            let _ = handle.join();
        }
    }
}

fn initialize_waveform_tracker(
    app: &mut MantleApp,
    device: &DeviceInfo,
    update_interval_ms: u64,
    mode: WaveformMode,
) {
    let device_id = device.id();

    let is_toggle_off = app
        .waveform_map
        .get(&device_id)
        .map(|w| w.active && w.mode == mode)
        .unwrap_or(false);

    let existing_region = app
        .waveform_map
        .get(&device_id)
        .map(|w| w.region.clone())
        .unwrap_or(RegionCaptureTarget::All);

    stop_active_waveform(app, device_id);

    if is_toggle_off {
        return;
    }

    ensure_waveform_channel(app, device_id);

    app.waveform_map.insert(
        device_id,
        WaveformTracker {
            active: true,
            last_update: Instant::now(),
            mode: mode.clone(),
            region: existing_region.clone(),
            stop_tx: None,
        },
    );

    let tx = match app.waveform_channel.get(&device_id) {
        Some(channel) => channel.tx.clone(),
        None => return,
    };
    let (stop_tx, stop_rx) = mpsc::channel::<()>();

    let handle = match mode {
        WaveformMode::Screencap => {
            let region = existing_region;
            let shared_subregion = if matches!(region, RegionCaptureTarget::Subregion(_)) {
                app.subregion_points.get(&device_id).cloned()
            } else {
                None
            };
            thread::spawn(move || {
                let screen_manager = match ScreencapManager::new() {
                    Ok(sm) => sm,
                    Err(e) => {
                        log::error!("Failed to create screen manager in capture thread: {}", e);
                        return;
                    }
                };
                loop {
                    let capture_region = match &shared_subregion {
                        Some(sub_lock) => {
                            let sub = sub_lock.lock().expect("Failed to lock subregion");
                            RegionCaptureTarget::Subregion(vec![sub.clone()])
                        }
                        None => region.clone(),
                    };
                    match screen_manager.calculate_average_color(capture_region) {
                        Ok(color) => {
                            if tx.send(color).is_err() {
                                break;
                            }
                        }
                        Err(e) => log::error!("Screen capture error: {}", e),
                    }
                    thread::sleep(Duration::from_millis(update_interval_ms));
                    if stop_rx.try_recv().is_ok() {
                        break;
                    }
                }
            })
        }
        WaveformMode::Audio => {
            let buffer_clone = app.audio_manager.clone_samples_buffer();
            thread::spawn(move || loop {
                let samples = match buffer_clone.lock() {
                    Ok(buf) => buf.clone(),
                    Err(_) => break,
                };
                let audio_color = AudioManager::samples_to_hsbk(samples);
                if tx.send(audio_color).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(update_interval_ms));
                if stop_rx.try_recv().is_ok() {
                    break;
                }
            })
        }
    };

    if let Some(channel) = app.waveform_channel.get_mut(&device_id) {
        channel.handle = Some(handle);
    }
    if let Some(tracker) = app.waveform_map.get_mut(&device_id) {
        tracker.stop_tx = Some(stop_tx);
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
    let mut latest = None;
    while let Ok(computed_color) = color_channel.rx.try_recv() {
        latest = Some(computed_color);
    }
    if let Some(latest_color) = latest {
        *color = Some(latest_color);
        tracker.last_update = Instant::now();
    }
}
