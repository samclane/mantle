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
    scenes::Scene,
    shortcut::{KeyboardShortcutAction, ShortcutEdit},
    HSBK32,
};

const DEFAULT_REFRESH_RATE_MS: u64 = 500;
const DEFAULT_UPDATE_INTERVAL_MS: u64 = 500;
const REFRESH_RATE_RANGE: std::ops::RangeInclusive<u64> = 50..=10_000;
const UPDATE_INTERVAL_MS_RANGE: std::ops::RangeInclusive<u64> = 50..=10_000;
const AUDIO_BUFFER_RANGE: std::ops::RangeInclusive<usize> = 1024..=AUDIO_BUFFER_DEFAULT;

#[derive(Deserialize, Serialize)]
pub struct Settings {
    pub custom_shortcuts: Vec<KeyboardShortcutAction>,
    pub refresh_rate_ms: u64,
    pub update_interval_ms: u64,
    pub scenes: Vec<Scene>,
    pub audio_buffer_size: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            custom_shortcuts: Vec::new(),
            refresh_rate_ms: DEFAULT_REFRESH_RATE_MS,
            update_interval_ms: DEFAULT_UPDATE_INTERVAL_MS,
            scenes: Vec::new(),
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
                .auto_sized()
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("Settings");
                    ui.separator();
                    ui.add_space(10.0);

                    self.render_refresh_rate(ui);

                    self.render_update_rate(ui);

                    self.render_audio_buffer_size(ui);

                    self.render_add_shortcut_ui(ui);

                    ui.separator();
                    self.render_scenes_ui(ui);
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
            if ui.button("Clear").clicked() {
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

            if ui.button("Add Shortcut").clicked() {
                self.settings
                    .custom_shortcuts
                    .push(self.shortcut_manager.new_shortcut.clone());
                self.shortcut_manager.add_shortcut(
                    self.shortcut_manager.new_shortcut.name.clone(),
                    self.shortcut_manager.new_shortcut.shortcut.clone(),
                    self.shortcut_manager.new_shortcut.action.clone(),
                    self.shortcut_manager.new_shortcut.device.clone().unwrap(),
                );
                // Clear the fields after adding
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
            }
        });
    }

    fn render_new_shortcut_grid(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("new_shortcut_grid").show(ui, |ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut self.shortcut_manager.new_shortcut.name);
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
                });
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
                });

            ui.label("Shortcut:");
            ui.add(ShortcutEdit::new(
                &mut self.shortcut_manager.new_shortcut.shortcut,
            ));
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
            );
        });
    }

    fn render_audio_buffer_size(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Audio Buffer Size:");
            ui.add(
                egui::Slider::new(&mut self.settings.audio_buffer_size, AUDIO_BUFFER_RANGE)
                    .text("samples"),
            );
        });
    }

    fn render_refresh_rate(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Refresh Rate:");
            ui.add(
                egui::Slider::new(&mut self.settings.refresh_rate_ms, REFRESH_RATE_RANGE)
                    .text("ms"),
            );
        });
    }

    fn render_scenes_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Scenes");
        ui.add_space(5.0);

        // Display existing scenes in a grid
        self.render_scenes_table(ui);

        ui.add_space(15.0);
        ui.separator();
        ui.add_space(10.0);
        ui.heading("Create New Scene");
        ui.add_space(5.0);

        self.render_new_scene_ui(ui);
    }

    fn render_scenes_table(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("scenes_grid").striped(true).show(ui, |ui| {
            ui.label(egui::RichText::new("Name").strong());
            ui.label(egui::RichText::new("Devices").strong());
            ui.label(egui::RichText::new("Actions").strong());
            ui.end_row();

            let mut to_remove = Vec::new();
            let mut applied = false;
            let mut application_errors = Vec::new();
            let mut removed = false;
            for scene in &self.settings.scenes {
                ui.label(&scene.name);
                ui.label(format!("{} devices", scene.device_color_pairs.len()));
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        // applied = scene.apply(&mut self.lighting_manager).is_ok();
                        match scene.apply(&mut self.lighting_manager) {
                            Ok(_) => {
                                applied = true;
                            }
                            Err(errors) => {
                                application_errors = errors;
                            }
                        }
                    }
                    if ui.button("Remove").clicked() {
                        to_remove.push(scene.name.clone());
                        removed = true;
                    }
                });
                ui.end_row();
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
        });
    }

    fn render_new_scene_ui(&mut self, ui: &mut egui::Ui) {
        // Provide UI to create a new scene
        ui.horizontal(|ui| {
            ui.label("Scene Name:");
            ui.text_edit_singleline(&mut self.new_scene.name);
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
                                .get_bulbs(&*self.lighting_manager.bulbs.lock().unwrap())
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
        if ui.button("Save Scene").clicked() {
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
