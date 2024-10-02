use std::sync::Arc;

use lifx_core::HSBK;
use serde::{Deserialize, Serialize};
use xcap::{
    image::{GenericImageView, RgbaImage},
    Monitor, Window, XCapError,
};

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
                Self::compare_by_id(m1, m2, |m| m.id().into())
            }
            (FollowType::Window(w1), FollowType::Window(w2)) => {
                Self::compare_by_id(w1, w2, |w| w.id().into())
            }
            (FollowType::Subregion(s1), FollowType::Subregion(s2)) => s1 == s2,
            (FollowType::All, FollowType::All) => true,
            _ => false,
        }
    }
}

impl FollowType {
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

impl PartialEq for ScreenSubregion {
    fn eq(&self, other: &Self) -> bool {
        self.monitor.as_ref().map(|m| m.id()) == other.monitor.as_ref().map(|m| m.id())
            && self.x == other.x
            && self.y == other.y
            && self.width == other.width
            && self.height == other.height
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

    pub fn from_click(&self, x: i32, y: i32) -> Result<HSBK, XCapError> {
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

    pub fn avg_color(&self, follow: FollowType) -> Result<HSBK, XCapError> {
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

        match follow {
            FollowType::Monitor(monitors) => {
                for monitor in monitors {
                    let image = monitor.capture_image()?;
                    calculate_image_pixel_average(&image);
                }
            }
            FollowType::Window(windows) => {
                for window in windows {
                    let image = window.capture_image()?;
                    calculate_image_pixel_average(&image);
                }
            }
            FollowType::Subregion(subregions) => {
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
            FollowType::All => {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_subregion_reset() {
        let mut subregion = ScreenSubregion {
            // monitor: Some(Arc::new(Monitor::primary().unwrap())),
            monitor: Some(Arc::new(Monitor::all().unwrap().first().unwrap().clone())),
            x: 100,
            y: 100,
            width: 200,
            height: 200,
        };
        subregion.reset();
        assert!(subregion.monitor.is_none());
        assert_eq!(subregion.x, 0);
        assert_eq!(subregion.y, 0);
        assert_eq!(subregion.width, 0);
        assert_eq!(subregion.height, 0);
    }

    #[test]
    fn test_screencap_manager_new() {
        let manager = ScreencapManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_screencap_manager_monitor_names() {
        let manager = ScreencapManager::new().unwrap();
        let names = manager.monitor_names();
        assert!(!names.is_empty());
    }

    #[test]
    fn test_screencap_manager_window_titles() {
        let manager = ScreencapManager::new().unwrap();
        let titles = manager.window_titles();
        assert!(!titles.is_empty());
    }

    #[test]
    fn test_screencap_manager_get_monitor() {
        let manager = ScreencapManager::new().unwrap();
        let names = manager.monitor_names();
        if let Some(name) = names.first() {
            let monitor = manager.get_monitor(name);
            assert!(monitor.is_some());
            assert_eq!(monitor.unwrap().name(), name);
        }
    }

    #[test]
    fn test_screencap_manager_get_window() {
        let manager = ScreencapManager::new().unwrap();
        let titles = manager.window_titles();
        if let Some(title) = titles.first() {
            let window = manager.get_window(title);
            assert!(window.is_some());
            assert_eq!(window.unwrap().title(), title);
        }
    }

    #[test]
    fn test_screencap_manager_from_click() {
        let manager = ScreencapManager::new().unwrap();
        let monitor = Monitor::all().unwrap().first().unwrap().clone();
        let x = monitor.x() + monitor.width() as i32 / 2;
        let y = monitor.y() + monitor.height() as i32 / 2;
        let color = manager.from_click(x, y);
        assert!(color.is_ok());
    }

    #[test]
    fn test_screencap_manager_bounding_box() {
        let manager = ScreencapManager::new().unwrap();
        let rect = manager.bounding_box();
        assert!(rect.width() > 0.0);
        assert!(rect.height() > 0.0);
    }

    #[test]
    fn test_screencap_manager_avg_color_all() {
        let manager = ScreencapManager::new().unwrap();
        let color = manager.avg_color(FollowType::All);
        assert!(color.is_ok());
    }

    #[test]
    fn test_screencap_manager_avg_color_monitor() {
        let manager = ScreencapManager::new().unwrap();
        let monitors = manager.monitors.clone();
        let color = manager.avg_color(FollowType::Monitor(monitors));
        assert!(color.is_ok());
    }

    #[test]
    fn test_screencap_manager_avg_color_window() {
        let manager = ScreencapManager::new().unwrap();
        let windows = manager.windows.clone();
        let color = manager.avg_color(FollowType::Window(windows));
        assert!(color.is_ok());
    }

    #[test]
    fn test_follow_type_equality() {
        let manager = ScreencapManager::new().unwrap();
        let monitors1 = manager.monitors.clone();
        let monitors2 = manager.monitors.clone();
        let follow1 = FollowType::Monitor(monitors1);
        let follow2 = FollowType::Monitor(monitors2);
        assert_eq!(follow1, follow2);
    }
}
