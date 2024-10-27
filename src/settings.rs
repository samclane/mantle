use eframe::egui::{self, Context};
use serde::{Deserialize, Serialize};

use crate::{
    action::UserAction,
    app::MantleApp,
    device_info::DeviceInfo,
    shortcut::{KeyboardShortcutAction, ShortcutEdit},
};

#[derive(Deserialize, Serialize)]
pub struct Settings {
    pub custom_shortcuts: Vec<KeyboardShortcutAction>,
    pub refresh_rate_ms: u64,
    pub follow_rate_ms: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            custom_shortcuts: Vec::new(),
            refresh_rate_ms: 500,
            follow_rate_ms: 500,
        }
    }
}
pub const SETTINGS_WINDOW_SIZE: [f32; 2] = [320.0, 480.0];

impl MantleApp {
    pub fn settings_ui(&mut self, ctx: &Context) {
        if self.show_settings {
            egui::Window::new("Settings")
                .default_size(SETTINGS_WINDOW_SIZE)
                .open(&mut self.show_settings)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.heading("Settings");
                    ui.separator();
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("Refresh Rate:");
                        ui.add(
                            egui::Slider::new(&mut self.settings.refresh_rate_ms, 50..=10_000)
                                .text("ms")
                                .clamp_to_range(true),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Follow Rate:");
                        ui.add(
                            egui::Slider::new(&mut self.settings.follow_rate_ms, 50..=10_000)
                                .text("ms")
                                .clamp_to_range(true),
                        );
                    });

                    ui.heading("Keyboard Shortcuts");
                    ui.add_space(5.0);

                    // Display existing shortcuts in a grid
                    egui::Grid::new("shortcuts_grid")
                        .striped(true)
                        .min_col_width(100.0)
                        .show(ui, |ui| {
                            ui.label(egui::RichText::new("Action").strong());
                            ui.label(egui::RichText::new("Shortcut").strong());
                            ui.end_row();

                            let mut to_remove = Vec::new();
                            for shortcut in self.settings.custom_shortcuts.iter() {
                                ui.label(&shortcut.name);
                                ui.label(&shortcut.shortcut.display_name);
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

                    ui.add_space(15.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.heading("Add New Shortcut");
                    ui.add_space(5.0);

                    egui::Grid::new("new_shortcut_grid")
                        .min_col_width(100.0)
                        .show(ui, |ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut self.shortcut_manager.new_shortcut.name);
                            ui.end_row();

                            ui.label("Action:");
                            egui::ComboBox::from_label("Action")
                                .selected_text(
                                    self.shortcut_manager.new_shortcut.action.to_string(),
                                )
                                .show_ui(ui, |ui| {
                                    for action in UserAction::variants() {
                                        if ui
                                            .selectable_label(
                                                self.shortcut_manager.new_shortcut.action
                                                    == *action,
                                                action.to_string(),
                                            )
                                            .clicked()
                                        {
                                            self.shortcut_manager.new_shortcut.action =
                                                action.clone();
                                        }
                                    }
                                });
                            ui.end_row();

                            ui.label("Device:");
                            egui::ComboBox::from_label("Device")
                                .selected_text(
                                    self.shortcut_manager
                                        .new_shortcut
                                        .device
                                        .clone()
                                        .unwrap_or(DeviceInfo::Group(self.mgr.all.clone()))
                                        .to_string(),
                                )
                                .show_ui(ui, |ui| {
                                    for device in self.mgr.bulbs.lock().unwrap().values() {
                                        ui.selectable_label(
                                            self.shortcut_manager
                                                .new_shortcut
                                                .device
                                                .clone()
                                                .unwrap_or(DeviceInfo::Group(self.mgr.all.clone()))
                                                == DeviceInfo::Bulb(Box::new(device.clone())),
                                            device.name.data.as_ref().unwrap().to_str().unwrap(),
                                        )
                                        .clicked()
                                        .then(|| {
                                            self.shortcut_manager.new_shortcut.device =
                                                Some(DeviceInfo::Bulb(Box::new(device.clone())));
                                        });
                                    }
                                    for group in self.mgr.get_groups() {
                                        ui.selectable_label(
                                            self.shortcut_manager
                                                .new_shortcut
                                                .device
                                                .clone()
                                                .unwrap_or(DeviceInfo::Group(self.mgr.all.clone()))
                                                == DeviceInfo::Group(group.clone()),
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

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("Clear").clicked() {
                            self.shortcut_manager.new_shortcut.name.clear();
                            self.shortcut_manager.new_shortcut.shortcut.keys.clear();
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
                                self.shortcut_manager.new_shortcut.action.clone(),
                                self.shortcut_manager.new_shortcut.device.clone().unwrap(),
                            );
                            // Clear the fields after adding
                            self.shortcut_manager.new_shortcut.name.clear();
                            self.shortcut_manager.new_shortcut.shortcut.keys.clear();
                            self.shortcut_manager
                                .new_shortcut
                                .shortcut
                                .update_display_string();
                        }
                    });
                });
        }
    }
}
