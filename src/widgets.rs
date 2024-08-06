use eframe::{
    egui::{self, Color32, Pos2, Sense, Shape, Stroke, Ui, Vec2, WidgetInfo, WidgetType},
    epaint::CubicBezierShape,
};

use crate::{AngleIter, BulbInfo, Manager, RGB};

pub fn display_color_circle(ui: &mut Ui, bulb: &BulbInfo, desired_size: Vec2, scale: f32) {
    let desired_size = ui.spacing().interact_size * desired_size;
    // Arc code from https://vcs.cozydsp.space/cozy-dsp/cozy-ui/src/commit/d4706ec9f4592137307ce8acafb56b881ea54e35/src/util.rs#L49
    if let Some(color) = bulb.get_color() {
        let rgb = RGB::from(*color);
        let (response, painter) = ui.allocate_painter(desired_size, Sense::hover());
        let center = response.rect.center();
        let radius = response.rect.width() / scale;
        let inner_stroke = Stroke::new(radius / 2.0, Color32::from(rgb));
        let outer_stroke = Stroke::new((5. / 6.) * radius, Color32::from_gray(64));
        let off_stroke = Stroke::new((5. / 6.) * radius, Color32::from_gray(32));
        let start_angle: f32 = 0.0;
        let end_angle: f32 = (2.0 * std::f32::consts::PI)
            * (bulb.get_color().unwrap().brightness as f32 / u16::MAX as f32);
        if bulb.power_level.data.unwrap() != 0.0 as u16 {
            painter.circle(center, radius, Color32::TRANSPARENT, outer_stroke);
            painter.extend(AngleIter::new(start_angle, end_angle).map(
                |(start_angle, end_angle)| {
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
                },
            ));
        } else {
            painter.circle(center, radius, Color32::TRANSPARENT, off_stroke);
        }
    }
}

pub fn toggle_button(ui: &mut Ui, mgr: &Manager, bulb: &BulbInfo, scale: Vec2) -> egui::Response {
    let desired_size = ui.spacing().interact_size * scale;
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());
    ui.horizontal(|ui| {
        if response.clicked() {
            if let Err(e) = mgr.toggle(&bulb) {
                println!("Error toggling bulb: {}", e);
            } else {
                println!("Toggled bulb {:?}", bulb.name);
            }
            response.mark_changed();
        }
        let on = bulb.power_level.data.unwrap_or(0.0 as u16) > 0;
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