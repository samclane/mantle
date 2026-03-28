use std::{
    collections::{HashMap, HashSet},
    ops::RangeInclusive,
    sync::MutexGuard,
};

use crate::{
    color::kelvin_to_rgb,
    contrast_color,
    device_info::DeviceInfo,
    products::{KELVIN_RANGE, LIFX_RANGE},
    AngleIter, BulbInfo, LifxManager, HSBK32, RGB8,
};

use eframe::{
    egui::{
        self, lerp, pos2, remap_clamp, vec2, Color32, Mesh, Pos2, Response, RichText, Sense, Shape,
        Stroke, Ui, Vec2, WidgetInfo, WidgetType,
    },
    epaint::CubicBezierShape,
};
use lifx_core::HSBK;

const SLIDER_RESOLUTION: u32 = 36;

pub fn create_highlighted_button(
    ui: &mut Ui,
    icon_name: &'static str,
    icon: &[u8],
    active: bool,
) -> Response {
    let active_color = ui.visuals().widgets.hovered.bg_stroke.color;
    let inactive_color = ui.visuals().widgets.inactive.bg_fill;

    let btn_id = ui.make_persistent_id(("hlbtn", icon_name));
    let prev_hovered: bool = ui.data(|d| d.get_temp(btn_id).unwrap_or(false));
    let hover_t = ui.ctx().animate_bool_responsive(btn_id, prev_hovered);
    let active_t = ui
        .ctx()
        .animate_bool_responsive(btn_id.with("active"), active);

    let fill = lerp_color32(inactive_color, active_color, active_t);
    let fill = lerp_color32(
        fill,
        brighten_color(fill, 40),
        hover_t * (1.0 - active_t * 0.5),
    );

    let response = ui.add(
        egui::Button::image(
            egui::Image::from_bytes(icon_name, icon.to_vec())
                .fit_to_exact_size(ui.spacing().interact_size),
        )
        .sense(Sense::click())
        .fill(fill),
    );

    ui.data_mut(|d| d.insert_temp(btn_id, response.hovered()));

    if hover_t > 0.01 {
        let glow_alpha = (30.0 * hover_t) as u8;
        ui.painter().rect_filled(
            response.rect.expand(2.0 * hover_t),
            ui.visuals().widgets.inactive.rounding,
            Color32::from_rgba_unmultiplied(180, 160, 220, glow_alpha),
        );
    }

    response
}

fn lerp_color32(a: Color32, b: Color32, t: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
        (a.a() as f32 + (b.a() as f32 - a.a() as f32) * t) as u8,
    )
}

fn brighten_color(c: Color32, amount: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(
        c.r().saturating_add(amount),
        c.g().saturating_add(amount),
        c.b().saturating_add(amount),
        c.a(),
    )
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
    let rgb = RGB8::from(color);
    let (response, painter) = ui.allocate_painter(desired_size, Sense::hover());
    let center = response.rect.center();
    let radius = response.rect.width() / scale;

    let power_id = response.id.with("power");
    let power_on = power != 0;
    let power_t = ui.ctx().animate_bool_responsive(power_id, power_on);

    let arc_id = response.id.with("brightness_arc");
    let target_angle: f32 =
        (2.0 * std::f32::consts::PI) * (color.brightness as f32 / u16::MAX as f32);
    let end_angle = ui.ctx().animate_value_with_time(arc_id, target_angle, 0.15);

    let arc_alpha = (255.0 * power_t) as u8;
    let arc_rgb = Color32::from(rgb);
    let inner_stroke = Stroke::new(
        radius / 2.0,
        Color32::from_rgba_unmultiplied(arc_rgb.r(), arc_rgb.g(), arc_rgb.b(), arc_alpha),
    );

    let off_gray = (32.0 + 32.0 * power_t) as u8;
    let bg_stroke = Stroke::new((5. / 6.) * radius, Color32::from_gray(off_gray));
    painter.circle(center, radius, Color32::TRANSPARENT, bg_stroke);

    if power_t > 0.01 {
        let start_angle: f32 = 0.0;
        let animated_end = end_angle * power_t;
        if animated_end > 0.001 {
            painter.extend(AngleIter::new(start_angle, animated_end).map(|(sa, ea)| {
                let xc = center.x;
                let yc = center.y;
                let p1 = center + radius * Vec2::new(sa.cos(), -sa.sin());
                let p4 = center + radius * Vec2::new(ea.cos(), -ea.sin());
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
            }));
        }
    }
}

pub fn toggle_button(
    ui: &mut Ui,
    lifx_manager: &LifxManager,
    device: &DeviceInfo,
    scale: Vec2,
    registered_bulbs: &mut MutexGuard<HashMap<u64, BulbInfo>>,
) -> egui::Response {
    let desired_size = ui.spacing().interact_size * scale;
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());
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
                    if let Some(live_bulb) = registered_bulbs.get_mut(&bulb.target) {
                        live_bulb.power_level.update(level);
                    }
                }
            }
            DeviceInfo::Group(group) => {
                if let Err(e) = lifx_manager.set_group_power(group, registered_bulbs, level) {
                    log::error!("Error toggling group: {}", e);
                } else {
                    log::info!("Toggled group {:?}", group.label);
                    let targets: Vec<u64> = group
                        .get_bulbs(registered_bulbs)
                        .iter()
                        .map(|b| b.target)
                        .collect();
                    for target in targets {
                        if let Some(live_bulb) = registered_bulbs.get_mut(&target) {
                            live_bulb.power_level.update(level);
                        }
                    }
                }
            }
        }
        response.mark_changed();
    }
    response
        .widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, ui.is_enabled(), on, "Toggle"));
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
    response
}

pub fn color_slider(
    ui: &mut Ui,
    value: &mut u16,
    range: std::ops::RangeInclusive<u16>,
    label: &str,
    get_color_at_value: impl Fn(u16) -> Color32,
) -> Response {
    let slider_width = (ui.available_width() - 70.0).max(60.0);
    let desired_size = vec2(slider_width, ui.spacing().interact_size.y);
    let (rect, response) = ui.allocate_at_least(desired_size, Sense::click_and_drag());

    let handle_radius = ui.spacing().slider_rail_height / 2.0 + 1.0;
    let handle_left = rect.left() + handle_radius;
    let handle_right = rect.right() - handle_radius;

    if let Some(mpos) = response.interact_pointer_pos() {
        *value = remap_clamp(
            mpos.x,
            handle_left..=handle_right,
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
            let half_h = ui.spacing().slider_rail_height / 2.0;
            let radius = half_h;
            let cy = rect.center().y;
            let cap_steps: u32 = 8;

            let mut mesh = Mesh::default();

            let left_color = get_color_at_value(0);
            let center_idx = mesh.vertices.len() as u32;
            mesh.colored_vertex(pos2(rect.left() + radius, cy), left_color);
            for j in 0..=cap_steps {
                let angle = std::f32::consts::FRAC_PI_2
                    + j as f32 * std::f32::consts::PI / cap_steps as f32;
                let px = rect.left() + radius + radius * angle.cos();
                let py = cy - radius * angle.sin();
                let vi = mesh.vertices.len() as u32;
                mesh.colored_vertex(pos2(px, py), left_color);
                if j > 0 {
                    mesh.add_triangle(center_idx, vi - 1, vi);
                }
            }

            let body_start = mesh.vertices.len() as u32;
            for i in 0..=SLIDER_RESOLUTION {
                let t = i as f32 / SLIDER_RESOLUTION as f32;
                let color = get_color_at_value((t * u16::MAX as f32) as u16);
                let x = lerp((rect.left() + radius)..=(rect.right() - radius), t);
                mesh.colored_vertex(pos2(x, cy + half_h), color);
                mesh.colored_vertex(pos2(x, cy - half_h), color);
                if i < SLIDER_RESOLUTION {
                    let bi = body_start + i * 2;
                    mesh.add_triangle(bi, bi + 1, bi + 2);
                    mesh.add_triangle(bi + 1, bi + 2, bi + 3);
                }
            }

            let right_color = get_color_at_value(u16::MAX);
            let center_idx = mesh.vertices.len() as u32;
            mesh.colored_vertex(pos2(rect.right() - radius, cy), right_color);
            for j in 0..=cap_steps {
                let angle = std::f32::consts::FRAC_PI_2
                    - j as f32 * std::f32::consts::PI / cap_steps as f32;
                let px = rect.right() - radius + radius * angle.cos();
                let py = cy - radius * angle.sin();
                let vi = mesh.vertices.len() as u32;
                mesh.colored_vertex(pos2(px, py), right_color);
                if j > 0 {
                    mesh.add_triangle(center_idx, vi - 1, vi);
                }
            }

            ui.painter().add(Shape::mesh(mesh));
        }

        let rail_rounding = rect.height() / 2.0;
        ui.painter()
            .rect_stroke(rect, rail_rounding, visuals.bg_stroke);

        {
            let x = lerp(
                handle_left..=handle_right,
                remap_clamp(
                    *value as f32,
                    RangeInclusive::new(*range.start() as f32, *range.end() as f32),
                    0.0..=1.0,
                ),
            );
            let dragging = response.is_pointer_button_down_on();
            let anim_t = ui
                .ctx()
                .animate_bool_responsive(response.id.with("handle_grow"), dragging);
            let radius = egui::lerp(handle_radius..=(handle_radius * 1.8), anim_t);
            let picked_color = get_color_at_value(*value);
            ui.painter().circle(
                pos2(x, rect.center().y),
                radius + 1.0,
                Color32::from_rgba_unmultiplied(0, 0, 0, 80),
                Stroke::NONE,
            );
            ui.painter().circle(
                pos2(x, rect.center().y),
                radius,
                picked_color,
                Stroke::new(visuals.fg_stroke.width + 0.5, contrast_color(picked_color)),
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

fn slider_label(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text)
            .size(12.0)
            .color(Color32::from_rgb(160, 160, 180)),
    );
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
            slider_label(ui, "Hue");
            hue_slider(ui, hue)
        });
        ui.horizontal(|ui| {
            slider_label(ui, "Saturation");
            saturation_slider(ui, saturation)
        });
        ui.horizontal(|ui| {
            slider_label(ui, "Brightness");
            brightness_slider(ui, brightness)
        });
        ui.horizontal(|ui| {
            slider_label(ui, "Kelvin");
            kelvin_slider(ui, kelvin, device)
        });
    })
    .response
}

/// Renders a horizontal strip of colored zone rectangles for multizone devices.
/// Returns the updated set of selected zone indices.
///
/// Supports click to select a single zone, Ctrl+click to toggle individual
/// zones, and click-and-drag to select a contiguous range.
pub fn zone_strip(
    ui: &mut Ui,
    zones: &[Option<HSBK>],
    selected: &HashSet<usize>,
) -> HashSet<usize> {
    let mut new_selected = selected.clone();
    let zone_count = zones.len();
    if zone_count == 0 {
        return new_selected;
    }

    let available_width = ui.available_width();
    let zone_width = (available_width / zone_count as f32).clamp(4.0, 24.0);
    let strip_height = 24.0;
    let total_width = zone_width * zone_count as f32;

    let (rect, response) = ui.allocate_exact_size(
        vec2(total_width, strip_height + 4.0),
        Sense::click_and_drag(),
    );

    let pos_to_zone = |pos: Pos2| -> usize {
        ((pos.x - rect.left()) / zone_width).clamp(0.0, (zone_count - 1) as f32) as usize
    };

    let drag_anchor_id = response.id.with("drag_anchor");

    if response.drag_started() {
        if let Some(pos) = response.interact_pointer_pos() {
            let anchor = pos_to_zone(pos);
            ui.data_mut(|d| d.insert_temp(drag_anchor_id, anchor));
        }
    }

    if response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let current = pos_to_zone(pos);
            let anchor: Option<usize> = ui.data(|d| d.get_temp(drag_anchor_id));
            if let Some(anchor) = anchor {
                let lo = anchor.min(current);
                let hi = anchor.max(current);
                new_selected = (lo..=hi).collect();
            }
        }
    }

    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let clicked_zone = pos_to_zone(pos);
            let modifiers = ui.input(|i| i.modifiers);
            if modifiers.ctrl || modifiers.command {
                if new_selected.contains(&clicked_zone) {
                    new_selected.remove(&clicked_zone);
                } else {
                    new_selected.insert(clicked_zone);
                }
            } else if new_selected.len() == 1 && new_selected.contains(&clicked_zone) {
                new_selected.clear();
            } else {
                new_selected.clear();
                new_selected.insert(clicked_zone);
            }
        }
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let rounding = egui::Rounding::same(3.0);
        painter.rect_filled(rect, rounding, Color32::from_rgb(20, 20, 28));

        for (i, zone_color) in zones.iter().enumerate() {
            let x_start = rect.left() + i as f32 * zone_width;
            let zone_rect = egui::Rect::from_min_size(
                pos2(x_start, rect.top() + 2.0),
                vec2(zone_width - 1.0, strip_height),
            );

            let fill = zone_color
                .map(|hsbk| Color32::from(RGB8::from(hsbk)))
                .unwrap_or(Color32::from_gray(40));

            painter.rect_filled(zone_rect, 2.0, fill);

            let sel_id = response.id.with(("zone_sel", i));
            let sel_t = ui
                .ctx()
                .animate_bool_responsive(sel_id, new_selected.contains(&i));
            if sel_t > 0.01 {
                let alpha = (255.0 * sel_t) as u8;
                painter.rect_stroke(
                    zone_rect.expand(sel_t),
                    2.0,
                    Stroke::new(
                        2.0 * sel_t,
                        Color32::from_rgba_unmultiplied(255, 200, 60, alpha),
                    ),
                );
            }
        }
    }

    new_selected
}
