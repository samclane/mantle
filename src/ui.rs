use std::{
    collections::HashMap,
    ops::RangeInclusive,
    sync::{mpsc, Arc, Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

use crate::{
    app::{
        MantleApp, RunningWaveform, EYEDROPPER_ICON, FOLLOW_RATE, ICON, MAIN_WINDOW_SIZE,
        MIN_WINDOW_SIZE, MONITOR_ICON, SUBREGION_ICON,
    },
    color::DeltaColor,
    contrast_color,
    device_info::DeviceInfo,
    screencap::{FollowType, ScreenSubregion, ScreencapManager},
    AngleIter, BulbInfo, Manager, RGB8,
};

use device_query::{DeviceQuery, DeviceState};
use eframe::{
    egui::{
        self, lerp, pos2, remap_clamp, vec2, Color32, Mesh, Pos2, Response, Sense, Shape, Stroke,
        Ui, Vec2, WidgetInfo, WidgetType,
    },
    epaint::CubicBezierShape,
};
use image::GenericImageView;
use lifx_core::HSBK;

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

pub fn handle_eyedropper(
    app: &mut MantleApp,
    ui: &mut Ui,
    device: &DeviceInfo,
) -> Option<DeltaColor> {
    let mut color: Option<HSBK> = None;
    // let mut show_eyedropper = app.show_eyedropper[&device.id()];
    let show_eyedropper = app.show_eyedropper.entry(device.id()).or_insert(false);
    let highlight = if *show_eyedropper {
        ui.visuals().widgets.hovered.bg_stroke.color
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };
    if ui
        .add(
            egui::Button::image(
                egui::Image::from_bytes("eyedropper", EYEDROPPER_ICON)
                    .fit_to_exact_size(Vec2::new(15., 15.)),
            )
            .sense(egui::Sense::click())
            .fill(highlight),
        )
        .clicked()
    {
        // show_eyedropper = !show_eyedropper;
        *show_eyedropper = !*show_eyedropper;
    }
    if *show_eyedropper {
        let screencap = ScreencapManager::new().unwrap();
        ui.ctx().output_mut(|out| {
            out.cursor_icon = egui::CursorIcon::Crosshair;
        });
        let device_state = DeviceState::new();
        let mouse = device_state.get_mouse();
        if mouse.button_pressed[1] {
            let position = mouse.coords;
            color = Some(screencap.from_click(position.0, position.1));
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
    #[cfg(debug_assertions)]
    puffin::profile_function!();
    let mut color: Option<HSBK> = None;
    let highlight = if app
        .waveform_map
        .get(&device.id())
        .map_or(false, |w| w.active)
    {
        ui.visuals().widgets.hovered.bg_stroke.color
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };
    if let Some((_tx, rx, _thread)) = app.waveform_trx.get(&device.id()) {
        let follow_state: &mut RunningWaveform =
            app.waveform_map
                .entry(device.id())
                .or_insert(RunningWaveform {
                    active: false,
                    last_update: Instant::now(),
                    follow_type: FollowType::All,
                    stop_tx: None,
                });
        if follow_state.active && (Instant::now() - follow_state.last_update > FOLLOW_RATE) {
            if let Ok(computed_color) = rx.try_recv() {
                color = Some(computed_color);
                follow_state.last_update = Instant::now();
            }
        }
    } else {
        let (tx, rx) = mpsc::channel();
        app.waveform_trx.insert(device.id(), (tx, rx, None));
    }

    if ui
        .add(
            egui::Button::image(
                egui::Image::from_bytes("monitor", MONITOR_ICON)
                    .fit_to_exact_size(Vec2::new(15., 15.)),
            )
            .sense(egui::Sense::click())
            .fill(highlight),
        )
        .clicked()
    {
        if let Some(waveform) = app.waveform_map.get_mut(&device.id()) {
            waveform.active = !waveform.active;
        } else {
            let running_waveform = RunningWaveform {
                active: true,
                last_update: Instant::now(),
                follow_type: FollowType::All,
                stop_tx: None,
            };
            app.waveform_map
                .insert(device.id(), running_waveform.clone());
        }
        // if the waveform is active, we need to spawn a thread to get the color
        if app.waveform_map[&device.id()].active {
            let screen_manager = app.screen_manager.clone();
            let tx = app.waveform_trx.get(&device.id()).unwrap().0.clone();
            let follow_type = app.waveform_map[&device.id()].follow_type.clone();
            let mgr = app.mgr.clone(); // Assuming you have a 'manager' field in MantleApp to control the bulb/group
            let device_id = device.id();

            let (stop_tx, stop_rx) = mpsc::channel::<()>();
            if let Some(waveform_trx) = app.waveform_trx.get_mut(&device.id()) {
                waveform_trx.2 = Some(thread::spawn(move || loop {
                    #[cfg(debug_assertions)]
                    puffin::profile_function!();

                    let avg_color = screen_manager.avg_color(follow_type.clone());

                    mgr.set_color_by_id(device_id, avg_color).unwrap();

                    if let Err(err) = tx.send(avg_color) {
                        eprintln!("Failed to send color data: {}", err);
                    }
                    thread::sleep(Duration::from_millis((FOLLOW_RATE.as_millis() / 4) as u64));
                    if stop_rx.try_recv().is_ok() {
                        break;
                    }
                }));
            }
            app.waveform_map.get_mut(&device.id()).unwrap().stop_tx = Some(stop_tx);
        } else {
            // kill thread
            if let Some(waveform_trx) = app.waveform_trx.get_mut(&device.id()) {
                if let Some(thread) = waveform_trx.2.take() {
                    // Send a signal to stop the thread
                    if let Some(stop_tx) = app
                        .waveform_map
                        .get_mut(&device.id())
                        .unwrap()
                        .stop_tx
                        .take()
                    {
                        stop_tx.send(()).unwrap();
                    }
                    // Wait for the thread to finish
                    thread.join().unwrap();
                }
            }
        }
    }
    if let Some(waveform) = app.waveform_map.get_mut(&device.id()) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.radio_value(&mut waveform.follow_type, FollowType::All, "All");
                for monitor in app.screen_manager.monitors.iter() {
                    ui.radio_value(
                        &mut waveform.follow_type,
                        FollowType::Monitor(vec![monitor.clone()]),
                        monitor.name(),
                    );
                }
            });

            ui.horizontal(|ui| {
                for window in app.screen_manager.windows.iter() {
                    ui.radio_value(
                        &mut waveform.follow_type,
                        FollowType::Window(vec![window.clone()]),
                        window.title(),
                    );
                }
            });

            let mut subregion = app
                .subregion_points
                .entry(device.id())
                .or_default()
                .lock()
                .unwrap();
            ui.radio_value(
                &mut waveform.follow_type,
                FollowType::Subregion(vec![subregion.clone()]),
                "Subregion",
            );
            // add numerical fields for subregion
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
        });
    }

    color.map(|color| DeltaColor {
        next: color,
        duration: Some((FOLLOW_RATE.as_millis() / 2) as u32),
    })
}

pub fn handle_get_subregion_bounds(app: &mut MantleApp, ui: &mut Ui, device_id: u64) {
    // Get or create the subregion
    let subregion_lock = app
        .subregion_points
        .entry(device_id)
        .or_insert_with(|| Arc::new(Mutex::new(ScreenSubregion::default())));
    let show_subregion = app.show_subregion.entry(device_id).or_insert(false);

    let mut subregion = subregion_lock.lock().unwrap();

    let highlight = if *show_subregion {
        ui.visuals().widgets.hovered.bg_stroke.color
    } else {
        ui.visuals().widgets.inactive.bg_fill
    };

    if ui
        .add(
            egui::Button::image(
                egui::Image::from_bytes("subregion", SUBREGION_ICON)
                    .fit_to_exact_size(Vec2::new(15., 15.)),
            )
            .sense(egui::Sense::click())
            .fill(highlight),
        )
        .clicked()
    {
        *show_subregion = !*show_subregion;
        if *show_subregion {
            subregion.reset();
        }
    }
    if *show_subregion {
        if app.input_listener.is_button_pressed(rdev::Button::Left) {
            let mouse_pos = app.input_listener.get_last_mouse_position().unwrap();
            if subregion.x == 0 && subregion.y == 0 {
                subregion.x = mouse_pos.0;
                subregion.y = mouse_pos.1;
            } else {
                subregion.width = (mouse_pos.0 - subregion.x).unsigned_abs();
                subregion.height = (mouse_pos.1 - subregion.y).unsigned_abs();
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
        DeviceInfo::Group(group) => group.any_on(bulbs) as u16 * u16::MAX,
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
    mgr: &Manager,
    device: &DeviceInfo,
    scale: Vec2,
    bulbs: &MutexGuard<HashMap<u64, BulbInfo>>,
) -> egui::Response {
    let desired_size = ui.spacing().interact_size * scale;
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());
    ui.horizontal(|ui| {
        let on = match device {
            DeviceInfo::Bulb(bulb) => bulb.power_level.data.unwrap_or(0) != 0,
            DeviceInfo::Group(group) => group.any_on(bulbs),
        };
        if response.clicked() {
            let level = if on { 0 } else { u16::MAX };
            match device {
                DeviceInfo::Bulb(bulb) => {
                    if let Err(e) = mgr.set_power(bulb, level) {
                        log::error!("Error toggling bulb: {}", e);
                    } else {
                        log::info!("Toggled bulb {:?}", bulb.name);
                    }
                }
                DeviceInfo::Group(group) => {
                    if let Err(e) = mgr.set_group_power(group, bulbs, level) {
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

const N: u32 = 6 * 6;

pub fn color_slider(
    ui: &mut Ui,
    value: &mut u16,
    range: std::ops::RangeInclusive<u16>,
    label: &str,
    color_at: impl Fn(u16) -> Color32,
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
            for i in 0..=N {
                let t = i as f32 / (N as f32);
                let color = color_at((t * u16::MAX as f32) as u16);
                let x = lerp(rect.left()..=rect.right(), t);
                // round edges:
                let y_offset = if i == 0 || i == N {
                    (ui.spacing().slider_rail_height / 2.0) - 2.
                } else {
                    ui.spacing().slider_rail_height / 2.0
                };
                mesh.colored_vertex(pos2(x, rect.center().y + y_offset), color);
                mesh.colored_vertex(pos2(x, rect.center().y - y_offset), color);
                if i < N {
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
            let r = ui.spacing().slider_rail_height / 1.3;
            let picked_color = color_at(*value);
            ui.painter().circle(
                pos2(x, rect.center().y), // center
                r,                        // radius
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
