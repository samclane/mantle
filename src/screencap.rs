use lifx_core::HSBK;
use xcap::{Monitor, Window, XCapError};

use crate::RGB8;

pub enum FollowType {
    // either monitor, window, or All
    Monitor(Vec<Monitor>),
    Window(Vec<Window>),
    All,
}

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
        let monitor = Monitor::from_point(x, y).unwrap();
        let new_x = x - monitor.x();
        let new_y = y - monitor.y();
        let rgba = *monitor
            .capture_image()
            .unwrap()
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
        match follow {
            FollowType::Monitor(monitors) => {
                for monitor in monitors {
                    let image = monitor.capture_image().unwrap();
                    for x in 0..monitor.width() {
                        for y in 0..monitor.height() {
                            let rgba = *image.get_pixel(x, y);
                            red += rgba[0] as u32;
                            green += rgba[1] as u32;
                            blue += rgba[2] as u32;
                            count += 1;
                        }
                    }
                }
            }
            FollowType::Window(windows) => {
                for window in windows {
                    let image = window.capture_image().unwrap();
                    for x in 0..window.width() {
                        for y in 0..window.height() {
                            let rgba = *image.get_pixel(x, y);
                            red += rgba[0] as u32;
                            green += rgba[1] as u32;
                            blue += rgba[2] as u32;
                            count += 1;
                        }
                    }
                }
            }
            FollowType::All => {
                for monitor in &self.monitors {
                    let image = monitor.capture_image().unwrap();
                    for x in 0..monitor.width() {
                        for y in 0..monitor.height() {
                            let rgba = *image.get_pixel(x, y);
                            red += rgba[0] as u32;
                            green += rgba[1] as u32;
                            blue += rgba[2] as u32;
                            count += 1;
                        }
                    }
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
