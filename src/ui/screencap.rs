use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use crate::{
    app::{
        MantleApp, WaveformMode, WaveformTracker, EYEDROPPER_ICON, MONITOR_ICON, SUBREGION_ICON,
    },
    color::DeltaColor,
    device_info::DeviceInfo,
    screencap::{RegionCaptureTarget, ScreenSubregion, ScreencapManager},
};

use super::{
    waveform::{
        ensure_waveform_channel, has_time_elapsed, initialize_waveform_tracker,
        update_color_from_channel,
    },
    widgets::create_highlighted_button,
};

use std::collections::HashMap;

use eframe::egui::{self, pos2, vec2, Color32, Pos2, Rect, Sense, Stroke, TextureHandle, Ui};
use lifx_core::HSBK;

const DEBOUNCE_DELAY_MS: u64 = 100;

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
        initialize_waveform_tracker(
            app,
            device,
            update_interval_ms,
            WaveformMode::Screencap,
            ui.ctx().clone(),
        );
    }
    color.map(|color| DeltaColor {
        next: color,
        duration: Some((update_interval_ms / 2) as u32),
    })
}

pub fn render_capture_target(app: &mut MantleApp, ui: &mut Ui, device: &DeviceInfo) {
    let update_interval_ms = app.settings.update_interval_ms;
    let mut region_type_changed = false;
    if let Some(waveform) = app.waveform_map.get_mut(&device.id()) {
        let prev_discriminant = std::mem::discriminant(&waveform.region);

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
            ui.horizontal(|ui| {
                ui.label("Capture Target");
                let combo_width = (ui.available_width() - 8.0).max(80.0);
                egui::ComboBox::from_id_salt("capture_target")
                    .selected_text(selected_text)
                    .width(combo_width)
                    .show_ui(ui, |ui| {
                        for (label, capture_target) in options {
                            ui.selectable_value(&mut waveform.region, capture_target, label);
                        }
                    });
            });
        });

        if let RegionCaptureTarget::Subregion(_) = waveform.region {
            let wide = ui.available_width() > 300.0;
            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::DragValue::new(&mut subregion.x));
                ui.label("Y:");
                ui.add(egui::DragValue::new(&mut subregion.y));
                if wide {
                    ui.label("Width:");
                    ui.add(egui::DragValue::new(&mut subregion.width));
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut subregion.height));
                }
            });
            if !wide {
                ui.horizontal(|ui| {
                    ui.label("Width:");
                    ui.add(egui::DragValue::new(&mut subregion.width));
                    ui.label("Height:");
                    ui.add(egui::DragValue::new(&mut subregion.height));
                });
            }
            render_subregion_preview(
                ui,
                &app.screen_manager,
                &mut subregion,
                &mut app.monitor_preview_textures,
            );
        }

        region_type_changed = waveform.active
            && waveform.mode == WaveformMode::Screencap
            && std::mem::discriminant(&waveform.region) != prev_discriminant;
    }
    if region_type_changed {
        initialize_waveform_tracker(
            app,
            device,
            update_interval_ms,
            WaveformMode::Screencap,
            ui.ctx().clone(),
        );
    }
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
    textures: &mut HashMap<u32, TextureHandle>,
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

    ui.horizontal(|ui| {
        ui.add_space(4.0);
        if ui.small_button("Refresh Preview").clicked() {
            textures.clear();
        }
    });

    let pad = 6.0;
    let preview_width = ui.available_width() - pad * 2.0;
    let scale = preview_width / total_width;
    let preview_height = total_height * scale;
    let outer_width = preview_width + pad * 2.0;
    let outer_height = preview_height + pad * 2.0;

    // Lazily capture and cache monitor screenshots as textures
    const PREVIEW_MAX_WIDTH: u32 = 480;
    for monitor in monitors {
        let mon_id = monitor.id();
        textures.entry(mon_id).or_insert_with(|| {
            let color_image = ScreencapManager::capture_monitor_preview(monitor, PREVIEW_MAX_WIDTH)
                .unwrap_or_else(|_| {
                    let w = (monitor.width() as f32 * scale).ceil() as usize;
                    let h = (monitor.height() as f32 * scale).ceil() as usize;
                    egui::ColorImage::new([w.max(1), h.max(1)], Color32::from_rgb(30, 30, 42))
                });
            ui.ctx().load_texture(
                format!("monitor_preview_{}", mon_id),
                color_image,
                egui::TextureOptions::LINEAR,
            )
        });
    }

    ui.add_space(4.0);
    let (response, painter) =
        ui.allocate_painter(vec2(outer_width, outer_height), Sense::click_and_drag());

    let outer_rect = response.rect;
    let inner_rect = outer_rect.shrink(pad);
    let origin = inner_rect.min;

    painter.rect(
        outer_rect,
        6.0,
        Color32::from_rgb(16, 16, 22),
        Stroke::new(1.5, Color32::from_rgb(100, 100, 130)),
    );

    let preview_to_global = |pos: Pos2| -> (i32, i32) {
        (
            ((pos.x - origin.x) / scale) as i32 + x_min,
            ((pos.y - origin.y) / scale) as i32 + y_min,
        )
    };

    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
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

        if let Some(tex) = textures.get(&monitor.id()) {
            painter.image(tex.id(), mon_rect, uv, Color32::WHITE);
        } else {
            painter.rect(mon_rect, 4.0, Color32::from_rgb(30, 30, 42), Stroke::NONE);
        }

        painter.rect_stroke(
            mon_rect,
            4.0,
            Stroke::new(1.0, Color32::from_rgb(55, 55, 75)),
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

        let time = ui.ctx().input(|i| i.time);
        let pulse = (time * 3.0).sin() as f32;
        let stroke_alpha = (180.0 + 75.0 * pulse) as u8;
        let fill_alpha = (25.0 + 15.0 * pulse) as u8;

        painter.rect(
            sub_rect,
            2.0,
            Color32::from_rgba_unmultiplied(180, 120, 30, fill_alpha),
            Stroke::new(
                2.0 + 0.5 * (pulse + 1.0) * 0.5,
                Color32::from_rgba_unmultiplied(220, 160, 50, stroke_alpha),
            ),
        );
        ui.ctx().request_repaint();
    }

    if response.hovered() {
        ui.ctx().output_mut(|out| {
            out.cursor_icon = egui::CursorIcon::Crosshair;
        });
    }
}
