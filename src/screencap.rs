use std::sync::Arc;

use lifx_core::HSBK;
use serde::{Deserialize, Serialize};
use xcap::{image::RgbaImage, Monitor, Window, XCapError};

use crate::RGB8;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ScreenSubregion {
    #[serde(skip)]
    pub monitor: Option<Arc<Monitor>>,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}
impl ScreenSubregion {
    pub fn reset(&mut self) {
        self.monitor = None;
        self.x = 0;
        self.y = 0;
        self.width = 0;
        self.height = 0;
    }
}

#[derive(Clone, Debug)]
pub enum FollowType {
    Monitor(Vec<Monitor>),
    Window(Vec<Window>),
    Subregion(Vec<ScreenSubregion>),
    All,
}

impl PartialEq for FollowType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FollowType::Monitor(m1), FollowType::Monitor(m2)) => {
                if m1.len() != m2.len() {
                    return false;
                }
                for (a, b) in m1.iter().zip(m2.iter()) {
                    if a.id() != b.id() {
                        return false;
                    }
                }
                true
            }
            (FollowType::Window(w1), FollowType::Window(w2)) => {
                if w1.len() != w2.len() {
                    return false;
                }
                for (a, b) in w1.iter().zip(w2.iter()) {
                    if a.id() != b.id() {
                        return false;
                    }
                }
                true
            }
            (FollowType::Subregion(s1), FollowType::Subregion(s2)) => {
                if s1.len() != s2.len() {
                    return false;
                }
                for (a, b) in s1.iter().zip(s2.iter()) {
                    if a.monitor.as_ref().map(|m| m.id()) != b.monitor.as_ref().map(|m| m.id())
                        || a.x != b.x
                        || a.y != b.y
                        || a.width != b.width
                        || a.height != b.height
                    {
                        return false;
                    }
                }
                true
            }
            (FollowType::All, FollowType::All) => true,
            _ => false,
        }
    }
}

#[derive(Clone)]
pub struct ScreencapManager {
    pub monitors: Vec<Monitor>,
    pub windows: Vec<Window>,
}

impl ScreencapManager {
    pub fn new() -> Result<Self, XCapError> {
        let monitors = Monitor::all()?;
        let windows = Window::all()?;
        Ok(Self { monitors, windows })
    }

    pub fn refresh(&mut self) -> Result<(), XCapError> {
        self.monitors = Monitor::all()?;
        self.windows = Window::all()?;
        Ok(())
    }

    pub fn monitor_names(&self) -> Vec<String> {
        self.monitors.iter().map(|m| m.name().to_string()).collect()
    }

    pub fn window_titles(&self) -> Vec<String> {
        self.windows.iter().map(|w| w.title().to_string()).collect()
    }

    pub fn get_monitor(&self, name: &str) -> Option<&Monitor> {
        self.monitors.iter().find(|m| m.name() == name)
    }

    pub fn get_window(&self, title: &str) -> Option<&Window> {
        self.windows.iter().find(|w| w.title() == title)
    }

    pub fn from_click(&self, x: i32, y: i32) -> HSBK {
        let monitor = Monitor::from_point(x, y).expect("Failed to get monitor from point");
        let new_x = x - monitor.x();
        let new_y = y - monitor.y();
        let rgba = *monitor
            .capture_image()
            .expect("Failed to capture image")
            .get_pixel(new_x as u32, new_y as u32);
        RGB8 {
            red: rgba[0],
            green: rgba[1],
            blue: rgba[2],
            temperature: None,
        }
        .into()
    }

    pub fn bounding_box(&self) -> eframe::egui::Rect {
        let mut x_min = i32::MAX;
        let mut y_min = i32::MAX;
        let mut x_max = i32::MIN;
        let mut y_max = i32::MIN;
        for monitor in &self.monitors {
            x_min = x_min.min(monitor.x());
            y_min = y_min.min(monitor.y());
            x_max = x_max.max(monitor.x() + monitor.width() as i32);
            y_max = y_max.max(monitor.y() + monitor.height() as i32);
        }
        eframe::egui::Rect::from_min_max(
            eframe::egui::Pos2::new(x_min as f32, y_min as f32),
            eframe::egui::Pos2::new(x_max as f32, y_max as f32),
        )
    }

    pub fn avg_color(&self, follow: FollowType) -> HSBK {
        let mut red: u32 = 0;
        let mut green: u32 = 0;
        let mut blue: u32 = 0;
        let mut count: u32 = 0;

        let mut calculate_image_pixel_average = |image: &RgbaImage, width: u32, height: u32| {
            for x in 0..width {
                for y in 0..height {
                    let rgba = *image.get_pixel(x, y);
                    red += rgba[0] as u32;
                    green += rgba[1] as u32;
                    blue += rgba[2] as u32;
                    count += 1;
                }
            }
        };

        match follow {
            FollowType::Monitor(monitors) => {
                for monitor in monitors {
                    let image = monitor.capture_image().expect("Failed to capture image");
                    calculate_image_pixel_average(&image, monitor.width(), monitor.height());
                }
            }
            FollowType::Window(windows) => {
                for window in windows {
                    let image = window.capture_image().expect("Failed to capture image");
                    calculate_image_pixel_average(&image, window.width(), window.height());
                }
            }
            FollowType::Subregion(subregions) => {
                for subregion in subregions {
                    let image = if let Some(monitor) = &subregion.monitor {
                        monitor.capture_image().expect("Failed to capture image")
                    } else {
                        // Handle the case when subregion.monitor is None
                        // For example, return a default image or handle the error
                        // This is just a placeholder, replace it with the appropriate code
                        RgbaImage::new(0, 0)
                    };
                    calculate_image_pixel_average(&image, subregion.width, subregion.height);
                }
            }
            FollowType::All => {
                for monitor in &self.monitors {
                    let image = monitor.capture_image().expect("Failed to capture image");
                    calculate_image_pixel_average(&image, monitor.width(), monitor.height());
                }
            }
        }

        RGB8 {
            red: (red / count) as u8,
            green: (green / count) as u8,
            blue: (blue / count) as u8,
            temperature: None,
        }
        .into()
    }
}
