use std::sync::Arc;

use lifx_core::HSBK;
use serde::{Deserialize, Serialize};
use xcap::{
    image::{GenericImageView, RgbaImage},
    Monitor, Window, XCapError,
};

use crate::RGB8;

/// A subregion of a screen that can be captured.
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

impl PartialEq for ScreenSubregion {
    fn eq(&self, other: &Self) -> bool {
        self.monitor.as_ref().map(|m| m.id()) == other.monitor.as_ref().map(|m| m.id())
            && self.x == other.x
            && self.y == other.y
            && self.width == other.width
            && self.height == other.height
    }
}

/// Abstraction for capturable regions of the screen.
#[derive(Clone, Debug)]
pub enum RegionCaptureTarget {
    Monitor(Vec<Monitor>),
    Window(Vec<Window>),
    Subregion(Vec<ScreenSubregion>),
    All,
}

impl PartialEq for RegionCaptureTarget {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RegionCaptureTarget::Monitor(m1), RegionCaptureTarget::Monitor(m2)) => {
                Self::compare_by_id(m1, m2, |m| m.id().into())
            }
            (RegionCaptureTarget::Window(w1), RegionCaptureTarget::Window(w2)) => {
                Self::compare_by_id(w1, w2, |w| w.id().into())
            }
            (RegionCaptureTarget::Subregion(s1), RegionCaptureTarget::Subregion(s2)) => s1 == s2,
            (RegionCaptureTarget::All, RegionCaptureTarget::All) => true,
            _ => false,
        }
    }
}

impl RegionCaptureTarget {
    /// Helper function to compare vectors of monitors/windows based on their IDs
    fn compare_by_id<T, F>(v1: &[T], v2: &[T], id_fn: F) -> bool
    where
        F: Fn(&T) -> u64,
    {
        if v1.len() != v2.len() {
            return false;
        }
        for (a, b) in v1.iter().zip(v2.iter()) {
            if id_fn(a) != id_fn(b) {
                return false;
            }
        }
        true
    }
}

/// Holds information about the monitors and windows on the system
/// that can be captured and analyzed for color information.
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

    /// Get the color of the pixel at the given coordinates.
    pub fn color_from_click(&self, x: i32, y: i32) -> Result<HSBK, XCapError> {
        let monitor = Monitor::from_point(x, y)?;
        let new_x = x - monitor.x();
        let new_y = y - monitor.y();
        let image = monitor.capture_image()?;
        let rgba = *image.get_pixel(new_x as u32, new_y as u32);
        Ok(RGB8 {
            red: rgba[0],
            green: rgba[1],
            blue: rgba[2],
            temperature: None,
        }
        .into())
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

    pub fn calculate_average_color(
        &self,
        capture_target: RegionCaptureTarget,
    ) -> Result<HSBK, XCapError> {
        let mut red: u64 = 0;
        let mut green: u64 = 0;
        let mut blue: u64 = 0;
        let mut count: u64 = 0;

        let mut calculate_image_pixel_average = |image: &RgbaImage| {
            for pixel in image.pixels() {
                red += pixel[0] as u64;
                green += pixel[1] as u64;
                blue += pixel[2] as u64;
                count += 1;
            }
        };

        match capture_target {
            RegionCaptureTarget::Monitor(monitors) => {
                for monitor in monitors {
                    let image = monitor.capture_image()?;
                    calculate_image_pixel_average(&image);
                }
            }
            RegionCaptureTarget::Window(windows) => {
                for window in windows {
                    let image = window.capture_image()?;
                    calculate_image_pixel_average(&image);
                }
            }
            RegionCaptureTarget::Subregion(subregions) => {
                for subregion in subregions {
                    if let Some(monitor) = &subregion.monitor {
                        let image = monitor.capture_image()?;
                        let sub_image = image.view(
                            subregion.x as u32,
                            subregion.y as u32,
                            subregion.width,
                            subregion.height,
                        );
                        calculate_image_pixel_average(&sub_image.to_image());
                    } else {
                        // Handle the case when subregion.monitor is None
                        // For now, we skip it
                        continue;
                    }
                }
            }
            RegionCaptureTarget::All => {
                for monitor in &self.monitors {
                    let image = monitor.capture_image()?;
                    calculate_image_pixel_average(&image);
                }
            }
        }

        if count == 0 {
            return Err(XCapError::new("No pixels to average"));
        }

        Ok(RGB8 {
            red: (red / count) as u8,
            green: (green / count) as u8,
            blue: (blue / count) as u8,
            temperature: None,
        }
        .into())
    }
}
