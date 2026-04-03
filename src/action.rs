use eframe::egui;
use lifx_core::HSBK;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use strum_macros::{AsRefStr, EnumIter, VariantNames};

use crate::{
    color::HSBKField,
    device_info::DeviceInfo,
    scenes::Scene,
    ui::{brightness_slider, hsbk_sliders, hue_slider, kelvin_slider, saturation_slider},
    LifxManager,
};

/// An action that can be performed in the UI
/// Primarily used for storing shortcut data
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, EnumIter, VariantNames, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum UserAction {
    Refresh,
    SetBrightness {
        brightness: u16,
    },
    SetColor {
        hue: u16,
        saturation: u16,
        brightness: u16,
        kelvin: u16,
    },
    SetHue {
        hue: u16,
    },
    SetKelvin {
        kelvin: u16,
    },
    SetPower {
        power: bool,
    },
    SetSaturation {
        saturation: u16,
    },
    SetScene {
        scene: Scene,
    },
    TogglePower,
}

/// Take a `Scene` and convert it into a `UserAction`
/// A `Scene` is a list of devices and their colors
impl From<Scene> for UserAction {
    fn from(scene: Scene) -> Self {
        Self::SetScene { scene }
    }
}

impl UserAction {
    /// Run the stored action, given the current `LifxManager` and `DeviceInfo`
    /// for the device the action is to be performed on
    pub fn execute(&self, mut lifx_manager: LifxManager, device: DeviceInfo) {
        match self {
            UserAction::Refresh => {
                if let Err(e) = lifx_manager.refresh() {
                    log::error!("Failed to refresh: {}", e);
                }
            }
            UserAction::TogglePower => match device {
                DeviceInfo::Group(group_info) => {
                    lifx_manager.toggle_group_power(group_info);
                }
                DeviceInfo::Bulb(bulb_info) => {
                    let level = if bulb_info.power_level.data.unwrap_or(0u16) > 0 {
                        0
                    } else {
                        u16::MAX
                    };
                    if let Err(e) = lifx_manager.set_power(&&*bulb_info, level) {
                        log::error!("Failed to set power: {}", e);
                    }
                }
            },
            UserAction::SetColor {
                hue,
                saturation,
                brightness,
                kelvin,
            } => {
                log::info!(
                    "Executing action: Set Color - H: {}, S: {}, B: {}, K: {}",
                    hue,
                    saturation,
                    brightness,
                    kelvin
                );
                match device {
                    DeviceInfo::Group(_group_info) => {
                        if let Err(e) = lifx_manager.set_group_color(
                            &_group_info,
                            HSBK {
                                hue: *hue,
                                saturation: *saturation,
                                brightness: *brightness,
                                kelvin: *kelvin,
                            },
                            &lifx_manager.bulbs.lock().unwrap(),
                            None,
                        ) {
                            log::error!("Failed to set group color: {}", e);
                        }
                    }
                    DeviceInfo::Bulb(bulb_info) => {
                        if let Err(e) = lifx_manager.set_color(
                            &&*bulb_info,
                            HSBK {
                                hue: *hue,
                                saturation: *saturation,
                                brightness: *brightness,
                                kelvin: *kelvin,
                            },
                            None,
                        ) {
                            log::error!("Failed to set color: {}", e);
                        }
                    }
                }
            }
            UserAction::SetPower { power } => {
                log::info!("Executing action: Set Power - {}", power);
                match device {
                    DeviceInfo::Group(_group_info) => {
                        let power = if *power { u16::MAX } else { 0 };
                        if let Err(e) = lifx_manager.set_group_power(
                            &_group_info,
                            &lifx_manager.bulbs.lock().unwrap(),
                            power,
                        ) {
                            log::error!("Failed to set group power: {}", e);
                        }
                    }
                    DeviceInfo::Bulb(bulb_info) => {
                        if let Err(e) = lifx_manager.set_power(&&*bulb_info, *power as u16) {
                            log::error!("Failed to set power: {}", e);
                        }
                    }
                }
            }
            UserAction::SetBrightness { brightness } => {
                log::info!("Executing action: Set Brightness - {}", brightness);
                match device {
                    DeviceInfo::Group(_group_info) => {
                        if let Err(e) = lifx_manager.set_group_color_field(
                            &_group_info,
                            HSBKField::Brightness,
                            *brightness,
                            &lifx_manager.bulbs.lock().unwrap(),
                        ) {
                            log::error!("Failed to set group brightness: {}", e);
                        }
                    }
                    DeviceInfo::Bulb(bulb_info) => {
                        if let Err(e) = lifx_manager.set_color_field(
                            &&*bulb_info,
                            HSBKField::Brightness,
                            *brightness,
                        ) {
                            log::error!("Failed to set brightness: {}", e);
                        }
                    }
                }
            }
            UserAction::SetSaturation { saturation } => {
                log::info!("Executing action: Set Saturation - {}", saturation);
                match device {
                    DeviceInfo::Group(_group_info) => {
                        if let Err(e) = lifx_manager.set_group_color_field(
                            &_group_info,
                            HSBKField::Saturation,
                            *saturation,
                            &lifx_manager.bulbs.lock().unwrap(),
                        ) {
                            log::error!("Failed to set group saturation: {}", e);
                        }
                    }
                    DeviceInfo::Bulb(bulb_info) => {
                        if let Err(e) = lifx_manager.set_color_field(
                            &&*bulb_info,
                            HSBKField::Saturation,
                            *saturation,
                        ) {
                            log::error!("Failed to set saturation: {}", e);
                        }
                    }
                }
            }
            UserAction::SetKelvin { kelvin } => {
                log::info!("Executing action: Set Kelvin - {}", kelvin);
                match device {
                    DeviceInfo::Group(_group_info) => {
                        if let Err(e) = lifx_manager.set_group_color_field(
                            &_group_info,
                            HSBKField::Kelvin,
                            *kelvin,
                            &lifx_manager.bulbs.lock().unwrap(),
                        ) {
                            log::error!("Failed to set group kelvin: {}", e);
                        }
                    }
                    DeviceInfo::Bulb(bulb_info) => {
                        if let Err(e) =
                            lifx_manager.set_color_field(&&*bulb_info, HSBKField::Kelvin, *kelvin)
                        {
                            log::error!("Failed to set kelvin: {}", e);
                        }
                    }
                }
            }
            UserAction::SetHue { hue } => {
                log::info!("Executing action: Set Hue - {}", hue);
                match device {
                    DeviceInfo::Group(_group_info) => {
                        if let Err(e) = lifx_manager.set_group_color_field(
                            &_group_info,
                            HSBKField::Hue,
                            *hue,
                            &lifx_manager.bulbs.lock().unwrap(),
                        ) {
                            log::error!("Failed to set group hue: {}", e);
                        }
                    }
                    DeviceInfo::Bulb(bulb_info) => {
                        if let Err(e) =
                            lifx_manager.set_color_field(&&*bulb_info, HSBKField::Hue, *hue)
                        {
                            log::error!("Failed to set hue: {}", e);
                        }
                    }
                }
            }
            UserAction::SetScene { scene } => {
                log::info!("Executing action: Set Scene - {}", scene.name);
                if let Err(e) = scene.apply(&mut lifx_manager) {
                    log::error!("Failed to apply scene: {:?}", e);
                }
            }
        }
    }

    /// Draw UI elements for the corresponding action
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        device: Option<DeviceInfo>,
        scenes: Vec<Scene>,
    ) -> egui::Response {
        match self {
            UserAction::Refresh => ui.label(""),
            UserAction::TogglePower => ui.label(""),
            UserAction::SetColor {
                hue,
                saturation,
                brightness,
                kelvin,
            } => {
                if let Some(device) = device {
                    hsbk_sliders(ui, hue, saturation, brightness, kelvin, &device)
                } else {
                    ui.label("No device selected")
                }
            }
            UserAction::SetPower { power } => ui
                .checkbox(power, "Power")
                .on_hover_text("Set power on or off"),
            UserAction::SetHue { hue } => hue_slider(ui, hue),
            UserAction::SetBrightness { brightness } => brightness_slider(ui, brightness),
            UserAction::SetSaturation { saturation } => saturation_slider(ui, saturation),
            UserAction::SetKelvin { kelvin } => {
                if let Some(ref device) = device {
                    kelvin_slider(ui, kelvin, device)
                } else {
                    ui.label("No device selected")
                }
            }
            UserAction::SetScene {
                scene: current_scene,
            } => {
                let current_name = current_scene.name.clone();
                egui::ComboBox::from_label("Scene")
                    .selected_text(&current_name)
                    .show_ui(ui, |ui| {
                        for s in scenes {
                            let is_selected = s.name == current_name;
                            if ui.selectable_label(is_selected, &s.name).clicked() {
                                *self = UserAction::SetScene { scene: s };
                            }
                        }
                    })
                    .response
                    .on_hover_text("Select a saved scene to apply")
            }
        }
    }
}

impl Display for UserAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserAction::Refresh => write!(f, "Refresh"),
            UserAction::TogglePower => write!(f, "Toggle Power"),
            UserAction::SetColor {
                hue,
                saturation,
                brightness,
                kelvin,
            } => write!(
                f,
                "Set Color: H: {}, S: {}, B: {}, K: {}",
                hue, saturation, brightness, kelvin
            ),
            UserAction::SetPower { power } => write!(f, "Set Power: {}", power),
            UserAction::SetHue { hue } => write!(f, "Set Hue: {}", hue),
            UserAction::SetSaturation { saturation } => {
                write!(f, "Set Saturation: {}", saturation)
            }
            UserAction::SetBrightness { brightness } => {
                write!(f, "Set Brightness: {}", brightness)
            }
            UserAction::SetKelvin { kelvin } => write!(f, "Set Kelvin: {}", kelvin),
            UserAction::SetScene { scene } => write!(f, "Set Scene: {}", scene.name),
        }
    }
}

impl From<UserAction> for String {
    fn from(action: UserAction) -> Self {
        action.to_string()
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use lifx_core::{LifxIdent, LifxString};
    use strum::IntoEnumIterator;

    use super::*;
    use crate::{color::DEFAULT_KELVIN, device_info::GroupInfo, LifxManager};

    #[test]
    fn test_user_action_display() {
        let action = UserAction::Refresh;
        assert_eq!(format!("{}", action), "Refresh");

        let action = UserAction::SetBrightness { brightness: 5 };
        assert_eq!(format!("{}", action), "Set Brightness: 5");

        let action = UserAction::SetColor {
            hue: 120,
            saturation: 8,
            brightness: 6,
            kelvin: DEFAULT_KELVIN,
        };
        assert_eq!(
            format!("{}", action),
            format!("Set Color: H: 120, S: 8, B: 6, K: {}", DEFAULT_KELVIN)
        );
    }

    #[test]
    fn test_user_action_variants() {
        let mut variants = UserAction::iter();
        assert!(variants.any(|v| v == UserAction::Refresh));
        assert!(variants.any(|v| v == UserAction::TogglePower));
    }

    #[test]
    fn test_discriminant_names() {
        assert_eq!(UserAction::Refresh.as_ref(), "refresh");
        assert_eq!(UserAction::TogglePower.as_ref(), "toggle_power");
    }

    #[test]
    fn test_user_action_execute() {
        let manager = LifxManager::new().unwrap();

        let action = UserAction::Refresh;
        action.execute(
            manager.clone(),
            DeviceInfo::Group(GroupInfo {
                group: LifxIdent([0; 16]),
                label: LifxString::new(&CString::new("TestGroup").unwrap()),
                updated_at: 0u64,
            }),
        );

        let action = UserAction::SetBrightness { brightness: 5 };
        action.execute(
            manager.clone(),
            DeviceInfo::Group(GroupInfo {
                group: LifxIdent([0; 16]),
                label: LifxString::new(&CString::new("TestGroup").unwrap()),
                updated_at: 0u64,
            }),
        );
    }

    #[test]
    fn display_toggle_power() {
        assert_eq!(format!("{}", UserAction::TogglePower), "Toggle Power");
    }

    #[test]
    fn display_set_power() {
        assert_eq!(
            format!("{}", UserAction::SetPower { power: true }),
            "Set Power: true"
        );
        assert_eq!(
            format!("{}", UserAction::SetPower { power: false }),
            "Set Power: false"
        );
    }

    #[test]
    fn display_set_hue() {
        assert_eq!(
            format!("{}", UserAction::SetHue { hue: 1000 }),
            "Set Hue: 1000"
        );
    }

    #[test]
    fn display_set_saturation() {
        assert_eq!(
            format!("{}", UserAction::SetSaturation { saturation: 500 }),
            "Set Saturation: 500"
        );
    }

    #[test]
    fn display_set_kelvin() {
        assert_eq!(
            format!("{}", UserAction::SetKelvin { kelvin: 4000 }),
            "Set Kelvin: 4000"
        );
    }

    #[test]
    fn display_set_scene() {
        let scene = Scene::new(vec![], "My Scene".to_string());
        assert_eq!(
            format!("{}", UserAction::SetScene { scene }),
            "Set Scene: My Scene"
        );
    }

    #[test]
    fn from_scene_produces_set_scene() {
        let scene = Scene::new(vec![], "Test".to_string());
        let action: UserAction = scene.clone().into();
        match action {
            UserAction::SetScene { scene: s } => assert_eq!(s.name, "Test"),
            _ => panic!("Expected SetScene variant"),
        }
    }

    #[test]
    fn from_user_action_to_string_matches_display() {
        let action = UserAction::SetBrightness { brightness: 42 };
        let display = format!("{}", action);
        let string: String = action.into();
        assert_eq!(display, string);
    }

    #[test]
    fn serde_round_trip_refresh() {
        let action = UserAction::Refresh;
        let json = serde_json::to_string(&action).unwrap();
        let back: UserAction = serde_json::from_str(&json).unwrap();
        assert_eq!(back, UserAction::Refresh);
    }

    #[test]
    fn serde_round_trip_toggle_power() {
        let action = UserAction::TogglePower;
        let json = serde_json::to_string(&action).unwrap();
        let back: UserAction = serde_json::from_str(&json).unwrap();
        assert_eq!(back, UserAction::TogglePower);
    }

    #[test]
    fn serde_round_trip_set_color() {
        let action = UserAction::SetColor {
            hue: 100,
            saturation: 200,
            brightness: 300,
            kelvin: 4000,
        };
        let json = serde_json::to_string(&action).unwrap();
        let back: UserAction = serde_json::from_str(&json).unwrap();
        assert_eq!(back, action);
    }

    #[test]
    fn serde_round_trip_set_power() {
        let action = UserAction::SetPower { power: true };
        let json = serde_json::to_string(&action).unwrap();
        let back: UserAction = serde_json::from_str(&json).unwrap();
        assert_eq!(back, action);
    }

    #[test]
    fn serde_round_trip_set_brightness() {
        let action = UserAction::SetBrightness { brightness: 12345 };
        let json = serde_json::to_string(&action).unwrap();
        let back: UserAction = serde_json::from_str(&json).unwrap();
        assert_eq!(back, action);
    }

    #[test]
    fn serde_round_trip_set_scene() {
        let scene = Scene::new(vec![], "TestScene".to_string());
        let action = UserAction::SetScene { scene };
        let json = serde_json::to_string(&action).unwrap();
        let back: UserAction = serde_json::from_str(&json).unwrap();
        assert_eq!(back, action);
    }
}
