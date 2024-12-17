use eframe::egui::{self, Context};
use serde::{Deserialize, Serialize};

use crate::{
    action::UserAction,
    app::MantleApp,
    device_info::DeviceInfo,
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
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            custom_shortcuts: Vec::new(),
            refresh_rate_ms: DEFAULT_REFRESH_RATE_MS,
            follow_rate_ms: DEFAULT_FOLLOW_RATE_MS,
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
                    self.shortcut_manager.new_shortcut.action,
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
}
