use std::ffi::CString;

use eframe::egui::{self, Context};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;

use crate::{
    action::UserAction,
    app::MantleApp,
    audio::AUDIO_BUFFER_DEFAULT,
    color::default_hsbk,
    device_info::DeviceInfo,
    scenes::{Scene, ScheduledScene},
    shortcut::{KeyboardShortcutAction, ShortcutEdit},
    HSBK32,
};

const DEFAULT_REFRESH_RATE_MS: u64 = 500;
const DEFAULT_UPDATE_INTERVAL_MS: u64 = 500;
const DEFAULT_TRANSITION_MS: u64 = 0;
const REFRESH_RATE_RANGE: std::ops::RangeInclusive<u64> = 50..=10_000;
const UPDATE_INTERVAL_MS_RANGE: std::ops::RangeInclusive<u64> = 50..=10_000;
const TRANSITION_MS_RANGE: std::ops::RangeInclusive<u64> = 0..=5_000;
const AUDIO_BUFFER_RANGE: std::ops::RangeInclusive<usize> = 1024..=AUDIO_BUFFER_DEFAULT;

#[derive(Deserialize, Serialize)]
pub struct Settings {
    pub custom_shortcuts: Vec<KeyboardShortcutAction>,
    pub refresh_rate_ms: u64,
    pub transition_duration_ms: u64,
    pub update_interval_ms: u64,
    pub scenes: Vec<Scene>,
    pub scheduled_scenes: Vec<ScheduledScene>,
    pub audio_buffer_size: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            custom_shortcuts: Vec::new(),
            refresh_rate_ms: DEFAULT_REFRESH_RATE_MS,
            transition_duration_ms: DEFAULT_TRANSITION_MS,
            update_interval_ms: DEFAULT_UPDATE_INTERVAL_MS,
            scenes: Vec::new(),
            scheduled_scenes: Vec::new(),
            audio_buffer_size: AUDIO_BUFFER_DEFAULT,
        }
    }
}

impl MantleApp {
    pub fn settings_ui(&mut self, ctx: &Context) {
        let mut show_settings = self.show_settings;

        if show_settings {
            egui::Window::new("Settings")
                .open(&mut show_settings)
                .vscroll(true)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.heading("Settings");
                    ui.separator();
                    ui.add_space(10.0);

                    self.render_refresh_rate(ui);

                    self.render_update_rate(ui);

                    self.render_transition_duration(ui);

                    self.render_audio_buffer_size(ui);

                    self.render_add_shortcut_ui(ui);

                    ui.separator();
                    self.render_scenes_ui(ui);

                    ui.separator();
                    self.render_scene_schedule_ui(ui);
                });

            self.show_settings = show_settings;
        }
    }

    fn render_add_shortcut_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Keyboard Shortcuts");
        ui.add_space(5.0);

        // Display existing shortcuts in a grid
        self.render_shortcuts_table(ui);

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(10.0);
        ui.heading("Add New Shortcut");
        ui.add_space(5.0);

        self.render_new_shortcut_grid(ui);

        ui.add_space(10.0);

        self.render_new_shortcut_field(ui);
    }

    fn render_new_shortcut_field(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui
                .button("Clear")
                .on_hover_text("Clear shortcut fields")
                .clicked()
            {
                self.shortcut_manager.new_shortcut.name.clear();
                self.shortcut_manager
                    .new_shortcut
                    .shortcut
                    .input_action_keys
                    .clear();
                self.shortcut_manager
                    .new_shortcut
                    .shortcut
                    .update_display_string();
                self.info_toast("Fields cleared");
            }

            if ui
                .button("Add Shortcut")
                .on_hover_text("Register this keyboard shortcut")
                .clicked()
            {
                if let Some(device) = self.shortcut_manager.new_shortcut.device.clone() {
                    self.settings
                        .custom_shortcuts
                        .push(self.shortcut_manager.new_shortcut.clone());
                    self.shortcut_manager.add_shortcut(
                        self.shortcut_manager.new_shortcut.name.clone(),
                        self.shortcut_manager.new_shortcut.shortcut.clone(),
                        self.shortcut_manager.new_shortcut.action.clone(),
                        device,
                    );
                    self.shortcut_manager.new_shortcut.name.clear();
                    self.shortcut_manager
                        .new_shortcut
                        .shortcut
                        .input_action_keys
                        .clear();
                    self.shortcut_manager
                        .new_shortcut
                        .shortcut
                        .update_display_string();
                    self.success_toast("Shortcut added successfully");
                } else {
                    self.error_toast("Please select a target device before adding a shortcut");
                }
            }
        });
    }

    fn render_new_shortcut_grid(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("new_shortcut_grid").show(ui, |ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut self.shortcut_manager.new_shortcut.name)
                .on_hover_text("Name to identify this shortcut");
            ui.end_row();

            egui::ComboBox::from_label("Action")
                .selected_text(self.shortcut_manager.new_shortcut.action.to_string())
                .show_ui(ui, |ui| {
                    for action in UserAction::iter() {
                        if ui
                            .selectable_label(
                                self.shortcut_manager.new_shortcut.action == action.clone(),
                                action.to_string(),
                            )
                            .clicked()
                        {
                            self.shortcut_manager.new_shortcut.action = action.clone();
                        }
                    }
                })
                .response
                .on_hover_text("Choose the action this shortcut performs");
            // based on selected action, show relevant fields
            self.shortcut_manager.new_shortcut.action.ui(
                ui,
                self.shortcut_manager.new_shortcut.device.clone(),
                self.settings.scenes.clone(),
            );
            ui.end_row();

            egui::ComboBox::from_label("Device")
                .selected_text(
                    self.shortcut_manager
                        .new_shortcut
                        .device
                        .clone()
                        .unwrap_or(DeviceInfo::Group(
                            self.lighting_manager.all_bulbs_group.clone(),
                        ))
                        .to_string(),
                )
                .show_ui(ui, |ui| {
                    for device in self.lighting_manager.bulbs.lock().unwrap().values() {
                        ui.selectable_label(
                            self.shortcut_manager.new_shortcut.device.clone().unwrap_or(
                                DeviceInfo::Group(self.lighting_manager.all_bulbs_group.clone()),
                            ) == DeviceInfo::Bulb(Box::new(device.clone())),
                            device.name.data.as_ref().unwrap().to_str().unwrap(),
                        )
                        .clicked()
                        .then(|| {
                            self.shortcut_manager.new_shortcut.device =
                                Some(DeviceInfo::Bulb(Box::new(device.clone())));
                        });
                    }
                    for group in self.lighting_manager.get_groups() {
                        ui.selectable_label(
                            self.shortcut_manager.new_shortcut.device.clone().unwrap_or(
                                DeviceInfo::Group(self.lighting_manager.all_bulbs_group.clone()),
                            ) == DeviceInfo::Group(group.clone()),
                            group.label.cstr().to_str().unwrap(),
                        )
                        .clicked()
                        .then(|| {
                            self.shortcut_manager.new_shortcut.device =
                                Some(DeviceInfo::Group(group.clone()));
                        });
                    }
                })
                .response
                .on_hover_text("Choose the target device or group");

            ui.label("Shortcut:");
            ui.add(ShortcutEdit::new(
                &mut self.shortcut_manager.new_shortcut.shortcut,
            ))
            .on_hover_text("Click to start recording a key combination");
            ui.end_row();
        });
    }

    fn render_shortcuts_table(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("shortcuts_grid")
            .striped(true)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Name").strong());
                ui.label(egui::RichText::new("Device").strong());
                ui.label(egui::RichText::new("Action").strong());
                ui.label(egui::RichText::new("Shortcut").strong());
                ui.label(egui::RichText::new("Remove").strong());
                ui.end_row();

                let mut to_remove = Vec::new();
                let mut show_toast = false;
                for shortcut in self.settings.custom_shortcuts.iter() {
                    ui.label(&shortcut.name);
                    ui.label(
                        shortcut
                            .device
                            .as_ref()
                            .unwrap_or(&DeviceInfo::Group(
                                self.lighting_manager.all_bulbs_group.clone(),
                            ))
                            .to_string(),
                    );
                    ui.label(shortcut.action.to_string());
                    ui.label(&shortcut.shortcut.name);
                    if ui
                        .button("Remove")
                        .on_hover_text("Remove this shortcut")
                        .clicked()
                    {
                        to_remove.push(shortcut.clone());
                        show_toast = true;
                    }
                    ui.end_row();
                }
                if show_toast {
                    self.info_toast("Shortcut removed");
                }
                for shortcut in to_remove {
                    self.shortcut_manager.remove_shortcut(shortcut.clone());
                    self.settings.custom_shortcuts.retain(|s| s != &shortcut);
                }
            });
    }

    fn render_update_rate(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Update Rate:");
            ui.add(
                egui::Slider::new(
                    &mut self.settings.update_interval_ms,
                    UPDATE_INTERVAL_MS_RANGE,
                )
                .text("ms"),
            )
            .on_hover_text("Interval between screen/audio color updates");
        });
    }

    fn render_transition_duration(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Transition Duration:");
            ui.add(
                egui::Slider::new(
                    &mut self.settings.transition_duration_ms,
                    TRANSITION_MS_RANGE,
                )
                .text("ms"),
            )
            .on_hover_text("How long color changes take to fade (0 = instant)");
        });
    }

    fn render_audio_buffer_size(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Audio Buffer Size:");
            ui.add(
                egui::Slider::new(&mut self.settings.audio_buffer_size, AUDIO_BUFFER_RANGE)
                    .text("samples"),
            )
            .on_hover_text("Number of audio samples per capture buffer");
        });
    }

    fn render_refresh_rate(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Refresh Rate:");
            ui.add(
                egui::Slider::new(&mut self.settings.refresh_rate_ms, REFRESH_RATE_RANGE)
                    .text("ms"),
            )
            .on_hover_text("How often to poll LIFX devices for status updates");
        });
    }

    fn render_scene_schedule_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Scene Schedule");
        ui.add_space(5.0);

        let scene_names: Vec<String> = self
            .settings
            .scenes
            .iter()
            .map(|s| s.name.clone())
            .collect();
        let mut to_remove = Vec::new();
        for (i, sched) in self.settings.scheduled_scenes.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.checkbox(&mut sched.enabled, "");
                egui::ComboBox::from_id_salt(format!("sched_scene_{}", i))
                    .selected_text(&sched.scene_name)
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        for name in &scene_names {
                            if ui
                                .selectable_label(sched.scene_name == *name, name)
                                .clicked()
                            {
                                sched.scene_name = name.clone();
                            }
                        }
                    });
                ui.label("at");
                ui.add(
                    egui::DragValue::new(&mut sched.hour)
                        .range(0..=23)
                        .suffix("h"),
                );
                ui.label(":");
                ui.add(
                    egui::DragValue::new(&mut sched.minute)
                        .range(0..=59)
                        .suffix("m"),
                );
                if ui.small_button("Remove").clicked() {
                    to_remove.push(i);
                }
            });
        }
        for i in to_remove.into_iter().rev() {
            self.settings.scheduled_scenes.remove(i);
        }

        if ui.small_button("Add Schedule").clicked() {
            self.settings
                .scheduled_scenes
                .push(ScheduledScene::default());
        }
    }

    fn render_scenes_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Scenes");
        ui.add_space(5.0);

        ui.horizontal(|ui| {
            if ui
                .button("Export Scenes")
                .on_hover_text("Save all scenes to a JSON file")
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("mantle_scenes.json")
                    .add_filter("JSON", &["json"])
                    .save_file()
                {
                    match serde_json::to_string_pretty(&self.settings.scenes) {
                        Ok(json) => match std::fs::write(&path, json) {
                            Ok(_) => self.success_toast("Scenes exported successfully"),
                            Err(e) => self.error_toast(&format!("Failed to write file: {}", e)),
                        },
                        Err(e) => self.error_toast(&format!("Failed to serialize scenes: {}", e)),
                    }
                }
            }
            if ui
                .button("Import Scenes")
                .on_hover_text("Load scenes from a JSON file")
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                {
                    match std::fs::read_to_string(&path) {
                        Ok(json) => match serde_json::from_str::<Vec<Scene>>(&json) {
                            Ok(imported) => {
                                let count = imported.len();
                                for scene in imported {
                                    if !self.settings.scenes.iter().any(|s| s.name == scene.name) {
                                        self.settings.scenes.push(scene);
                                    }
                                }
                                self.success_toast(&format!("Imported {} scene(s)", count));
                            }
                            Err(e) => self.error_toast(&format!("Invalid scene file: {}", e)),
                        },
                        Err(e) => self.error_toast(&format!("Failed to read file: {}", e)),
                    }
                }
            }
        });
        ui.add_space(5.0);

        self.render_scenes_table(ui);

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(10.0);
        ui.heading("Create New Scene");
        ui.add_space(5.0);

        self.render_new_scene_ui(ui);
    }

    fn render_scenes_table(&mut self, ui: &mut egui::Ui) {
        let mut to_remove = Vec::new();
        let mut applied = false;
        let mut application_errors = Vec::new();
        let mut removed = false;

        for scene in &self.settings.scenes {
            let header_id = ui.make_persistent_id(format!("scene_{}", scene.name));
            egui::collapsing_header::CollapsingState::load_with_default_open(
                ui.ctx(),
                header_id,
                false,
            )
            .show_header(ui, |ui| {
                ui.label(egui::RichText::new(&scene.name).strong());
                ui.label(format!("{} devices", scene.device_color_pairs.len()));
                if ui
                    .button("Apply")
                    .on_hover_text("Apply this scene to your lights")
                    .clicked()
                {
                    match scene.apply(&mut self.lighting_manager) {
                        Ok(_) => {
                            applied = true;
                        }
                        Err(errors) => {
                            application_errors = errors;
                        }
                    }
                }
                if ui
                    .button("Remove")
                    .on_hover_text("Delete this scene")
                    .clicked()
                {
                    to_remove.push(scene.name.clone());
                    removed = true;
                }
            })
            .body(|ui| {
                for (device, color) in &scene.device_color_pairs {
                    ui.horizontal(|ui| {
                        let swatch_size = egui::vec2(14.0, 14.0);
                        let (response, painter) =
                            ui.allocate_painter(swatch_size, egui::Sense::hover());
                        let center = response.rect.center();
                        let radius = swatch_size.x / 2.0;
                        let swatch_color = egui::Color32::from(*color);
                        painter.circle_filled(center, radius, swatch_color);
                        painter.circle_stroke(
                            center,
                            radius,
                            egui::Stroke::new(1.0, ui.visuals().text_color()),
                        );

                        let prefix = match device {
                            DeviceInfo::Bulb(_) => "Bulb",
                            DeviceInfo::Group(_) => "Group",
                        };
                        let name = device.name().unwrap_or_else(|| "Unknown".to_string());
                        ui.label(format!("{prefix}: {name}"));
                    });
                }
            });
        }

        if applied {
            self.success_toast("Scene applied successfully");
        } else if !application_errors.is_empty() {
            self.error_toast(&format!(
                "Failed to apply scene: {}",
                application_errors.join(", ")
            ));
        }
        for name in to_remove {
            self.settings.scenes.retain(|s| s.name != name);
        }
        if removed {
            self.info_toast("Scene removed");
        }
    }

    fn render_new_scene_ui(&mut self, ui: &mut egui::Ui) {
        // Provide UI to create a new scene
        ui.horizontal(|ui| {
            ui.label("Scene Name:");
            ui.text_edit_singleline(&mut self.new_scene.name)
                .on_hover_text("Enter a name for the new scene");
        });
        ui.add_space(5.0);
        ui.label("Select Devices:");
        // Display list of devices with checkboxes
        egui::ScrollArea::vertical()
            .max_height(150.0)
            .show(ui, |ui| {
                for device in self.lighting_manager.bulbs.lock().unwrap().values() {
                    let mut selected = self
                        .new_scene
                        .devices_mut()
                        .any(|d| *d == DeviceInfo::Bulb(Box::new(device.clone())));
                    if ui
                        .checkbox(
                            &mut selected,
                            device
                                .name
                                .data
                                .as_ref()
                                .unwrap_or(&CString::default())
                                .to_str()
                                .unwrap_or("Unknown Device"),
                        )
                        .on_hover_text("Select device for the scene")
                        .changed()
                    {
                        if selected {
                            self.new_scene.device_color_pairs.push((
                                DeviceInfo::Bulb(Box::new(device.clone())),
                                (*device.get_color().unwrap_or(&default_hsbk())).into(),
                            ));
                        } else {
                            self.new_scene
                                .device_color_pairs
                                .retain(|(d, _)| *d != DeviceInfo::Bulb(Box::new(device.clone())));
                        }
                    }
                }
                // Add groups to the scene
                for group in self.lighting_manager.get_groups() {
                    let mut selected = self
                        .new_scene
                        .devices_mut()
                        .any(|d| *d == DeviceInfo::Group(group.clone()));
                    if ui
                        .checkbox(&mut selected, group.label.cstr().to_str().unwrap())
                        .on_hover_text("Select group for the scene")
                        .changed()
                    {
                        if selected {
                            self.new_scene
                                .device_color_pairs
                                .push((DeviceInfo::Group(group.clone()), default_hsbk().into()));

                            // Also add individual devices from the group
                            for device in group
                                .get_bulbs(&self.lighting_manager.bulbs.lock().unwrap())
                                .iter()
                            {
                                // Avoid duplicating devices
                                if !self.new_scene.devices_mut().any(|d| {
                                    if let DeviceInfo::Bulb(existing) = d {
                                        **existing == **device
                                    } else {
                                        false
                                    }
                                }) {
                                    self.new_scene.device_color_pairs.push((
                                        DeviceInfo::Bulb(Box::new((*device).clone())),
                                        (*device.get_color().unwrap_or(&default_hsbk())).into(),
                                    ));
                                }
                            }
                        } else {
                            // Remove the group
                            self.new_scene
                                .device_color_pairs
                                .retain(|(d, _)| *d != DeviceInfo::Group(group.clone()));

                            // Keep individual devices if they're explicitly selected
                            // We'll leave those alone
                        }
                    }
                }
            });
        ui.add_space(5.0);
        if ui
            .button("Save Scene")
            .on_hover_text("Save the current device colors as a scene")
            .clicked()
        {
            // Save the new scene
            let device_color_pairs = self
                .new_scene
                .devices()
                .iter()
                .map(|device| {
                    let color = device
                        .color()
                        .cloned()
                        .unwrap_or_else(crate::color::default_hsbk);
                    ((*device).clone(), HSBK32::from(color))
                })
                .collect();
            let scene = Scene {
                name: self.new_scene.name.clone(),
                device_color_pairs,
            };
            self.settings.scenes.push(scene);
            // Clear the new scene input
            self.new_scene.name.clear();
            self.new_scene.devices().clear();
            self.success_toast("Scene saved successfully");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_default_refresh_rate() {
        let settings = Settings::default();
        assert_eq!(settings.refresh_rate_ms, 500);
    }

    #[test]
    fn settings_default_update_interval() {
        let settings = Settings::default();
        assert_eq!(settings.update_interval_ms, 500);
    }

    #[test]
    fn settings_default_audio_buffer() {
        let settings = Settings::default();
        assert_eq!(settings.audio_buffer_size, AUDIO_BUFFER_DEFAULT);
    }

    #[test]
    fn settings_default_empty_shortcuts() {
        let settings = Settings::default();
        assert!(settings.custom_shortcuts.is_empty());
    }

    #[test]
    fn settings_default_empty_scenes() {
        let settings = Settings::default();
        assert!(settings.scenes.is_empty());
    }

    #[test]
    fn settings_serde_round_trip() {
        let settings = Settings {
            custom_shortcuts: Vec::new(),
            refresh_rate_ms: 1000,
            transition_duration_ms: 0,
            update_interval_ms: 2000,
            scenes: Vec::new(),
            scheduled_scenes: Vec::new(),
            audio_buffer_size: 4096,
        };
        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.refresh_rate_ms, 1000);
        assert_eq!(deserialized.update_interval_ms, 2000);
        assert_eq!(deserialized.audio_buffer_size, 4096);
        assert!(deserialized.custom_shortcuts.is_empty());
        assert!(deserialized.scenes.is_empty());
    }

    #[test]
    fn settings_serde_round_trip_default() {
        let settings = Settings::default();
        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.refresh_rate_ms, settings.refresh_rate_ms);
        assert_eq!(deserialized.update_interval_ms, settings.update_interval_ms);
        assert_eq!(deserialized.audio_buffer_size, settings.audio_buffer_size);
    }
}
