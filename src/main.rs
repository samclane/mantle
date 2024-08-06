use eframe::{
    egui::{self, Color32, Pos2, Sense, Shape, Slider, Stroke, Ui, Vec2, WidgetInfo, WidgetType},
    epaint::CubicBezierShape,
};
use lifx_core::HSBK;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

use mantle::{AngleIter, BulbInfo, Manager, RGB};

const SIZE: [f32; 2] = [320.0, 800.0];
const MIN_SIZE: [f32; 2] = [300.0, 220.0];
const LIFX_RANGE: std::ops::RangeInclusive<u16> = 0..=u16::MAX;

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(SIZE)
            .with_min_inner_size(MIN_SIZE),
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
}

impl Default for MantleApp {
    fn default() -> Self {
        let mgr = Manager::new().unwrap();
        Self { mgr }
    }
}

impl MantleApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }
}

impl eframe::App for MantleApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if Instant::now() - self.mgr.last_discovery > Duration::from_secs(10) {
            self.mgr.discover().unwrap();
        }
        self.mgr.refresh();
        egui::CentralPanel::default().show(_ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Devices");
                let bulbs = self.mgr.bulbs.lock();
                ui.vertical(|ui| {
                    if let Ok(bulbs) = bulbs {
                        let bulbs = bulbs.values();
                        for bulb in bulbs {
                            if let Some(s) = bulb.name.data.as_ref().and_then(|s| s.to_str().ok()) {
                                ui.label(s);
                            }
                            if let Some(g) = bulb
                                .group
                                .data
                                .as_ref()
                                .and_then(|g| g.label.cstr().to_str().ok())
                            {
                                ui.label(format!("Group: {}", g));
                            }

                            ui.horizontal(|ui| {
                                display_color_circle(ui, bulb, Vec2::new(1.0, 1.0), 8.0);

                                ui.vertical(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("Power");
                                        toggle_button(ui, &self.mgr, bulb, Vec2::new(1.0, 1.0));
                                    });
                                    if let Some(color) = bulb.get_color() {
                                        ui.vertical(|ui| {
                                            let HSBK {
                                                mut hue,
                                                mut saturation,
                                                mut brightness,
                                                mut kelvin,
                                            } = color;
                                            ui.add(Slider::new(&mut hue, LIFX_RANGE).text("Hue"));
                                            ui.add(
                                                Slider::new(&mut saturation, LIFX_RANGE)
                                                    .text("Saturation"),
                                            );
                                            ui.add(
                                                Slider::new(&mut brightness, LIFX_RANGE)
                                                    .text("Brightness"),
                                            );
                                            if let Some(range) =
                                                bulb.features.temperature_range.as_ref()
                                            {
                                                if range.min != range.max {
                                                    ui.add(
                                                        Slider::new(
                                                            &mut kelvin,
                                                            range.to_range_u16(),
                                                        )
                                                        .text("Kelvin"),
                                                    );
                                                } else {
                                                    ui.label(format!("Kelvin: {:?}", range.min));
                                                }
                                            }
                                            match self.mgr.set_color(
                                                &bulb,
                                                HSBK {
                                                    hue,
                                                    saturation,
                                                    brightness,
                                                    kelvin,
                                                },
                                            ) {
                                                Ok(_) => (),
                                                Err(e) => {
                                                    println!("Error setting brightness: {}", e)
                                                }
                                            }
                                        });
                                    }
                                });
                            });
                            ui.separator();
                        }
                    }
                });
            });
        });
    }
}

fn display_color_circle(ui: &mut Ui, bulb: &BulbInfo, desired_size: Vec2, scale: f32) {
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

fn toggle_button(ui: &mut Ui, mgr: &Manager, bulb: &BulbInfo, scale: Vec2) -> egui::Response {
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
