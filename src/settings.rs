use std::ffi::CString;

use eframe::egui::{self, Context};
use serde::{Deserialize, Serialize};

use crate::{
    action::UserAction,
    app::MantleApp,
    color::default_hsbk,
    device_info::DeviceInfo,
    scenes::Scene,
    shortcut::{KeyboardShortcutAction, ShortcutEdit},
};

const DEFAULT_REFRESH_RATE_MS: u64 = 500;
const DEFAULT_FOLLOW_RATE_MS: u64 = 500;
const REFRESH_RATE_RANGE: std::ops::RangeInclusive<u64> = 50..=10_000;
const FOLLOW_RATE_RANGE: std::ops::RangeInclusive<u64> = 50..=10_000;

#[derive(Deserialize, Serialize)]
pub struct Settings {
    pub custom_shortcuts: Vec<KeyboardShortcutAction>,
    pub refresh_rate_ms: u64,
    pub follow_rate_ms: u64,
    pub scenes: Vec<Scene>, // Add this line
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            custom_shortcuts: Vec::new(),
            refresh_rate_ms: DEFAULT_REFRESH_RATE_MS,
            follow_rate_ms: DEFAULT_FOLLOW_RATE_MS,
            scenes: Vec::new(), // Initialize scenes
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

                    self.render_follow_rate(ui);

                    self.render_add_shortcut_ui(ui);

                    ui.separator(); // Add separator
                    self.render_scenes_ui(ui); // Add this line
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
            }

            if ui.button("Add Shortcut").clicked() {
                self.settings
                    .custom_shortcuts
                    .push(self.shortcut_manager.new_shortcut.clone());
                self.shortcut_manager.add_shortcut(
                    self.shortcut_manager.new_shortcut.name.clone(),
                    self.shortcut_manager.new_shortcut.shortcut.clone(),
                    self.shortcut_manager.new_shortcut.action,
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
                    for action in UserAction::variants() {
                        if ui
                            .selectable_label(
                                self.shortcut_manager.new_shortcut.action == *action,
                                action.to_string(),
                            )
                            .clicked()
                        {
                            self.shortcut_manager.new_shortcut.action = *action;
                        }
                    }
                });
            // based on selected action, show relevant fields
            self.shortcut_manager
                .new_shortcut
                .action
                .ui(ui, self.shortcut_manager.new_shortcut.device.clone());
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
                for shortcut in self.settings.custom_shortcuts.iter() {
                    ui.label(&shortcut.name);
                    ui.label(shortcut.device.as_ref().unwrap().to_string());
                    ui.label(shortcut.action.to_string());
                    ui.label(&shortcut.shortcut.name);
                    if ui
                        .button("Remove")
                        .on_hover_text("Remove this shortcut")
                        .clicked()
                    {
                        to_remove.push(shortcut.clone());
                    }
                    ui.end_row();
                }
                for shortcut in to_remove {
                    self.shortcut_manager.remove_shortcut(shortcut.clone());
                    self.settings.custom_shortcuts.retain(|s| s != &shortcut);
                }
            });
    }

    fn render_follow_rate(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Follow Rate:");
            ui.add(
                egui::Slider::new(&mut self.settings.follow_rate_ms, FOLLOW_RATE_RANGE)
                    .text("ms")
                    .clamp_to_range(true),
            );
        });
    }

    fn render_refresh_rate(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Refresh Rate:");
            ui.add(
                egui::Slider::new(&mut self.settings.refresh_rate_ms, REFRESH_RATE_RANGE)
                    .text("ms")
                    .clamp_to_range(true),
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
            for scene in &self.settings.scenes {
                ui.label(&scene.name);
                ui.label(format!("{} devices", scene.device_color_pairs.len()));
                ui.horizontal(|ui| {
                    if ui.button("Apply").clicked() {
                        scene.apply(&mut self.lighting_manager);
                    }
                    if ui.button("Remove").clicked() {
                        to_remove.push(scene.name.clone());
                    }
                });
                ui.end_row();
            }
            for name in to_remove {
                self.settings.scenes.retain(|s| s.name != name);
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
                // TODO: Add groups to the scene
                // for group in self.mgr.get_groups() {
                //     let mut selected = self
                //         .new_scene
                //         .devices_mut()
                //         .any(|d| *d == DeviceInfo::Group(group.clone()));
                //     if ui
                //         .checkbox(&mut selected, group.label.cstr().to_str().unwrap())
                //         .on_hover_text("Select group for the scene")
                //         .changed()
                //     {
                //         if selected {
                //             for device in group.get_bulbs(&*self.mgr.bulbs.lock().unwrap()).iter() {
                //                 self.new_scene.device_color_pairs.push((
                //                     DeviceInfo::Bulb(Box::new((*device).clone())),
                //                     (*device.get_color().unwrap_or(&default_hsbk())).into(),
                //                 ));
                //             }
                //         } else {
                //             self.new_scene
                //                 .device_color_pairs
                //                 .retain(|(d, _)| *d != DeviceInfo::Group(group.clone()));
                //         }
                //     }
                // }
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
                    ((*device).clone(), color.into())
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
        }
    }
}
