use eframe::egui;
use lifx_core::HSBK;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

use crate::{
    color::HSBKField,
    device_info::DeviceInfo,
    ui::{brightness_slider, hsbk_sliders, hue_slider, kelvin_slider, saturation_slider},
    LifxManager,
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum UserAction {
    Refresh,
    TogglePower,
    SetColor {
        hue: u16,
        saturation: u16,
        brightness: u16,
        kelvin: u16,
    },
    SetPower {
        power: bool,
    },
    SetHue {
        hue: u16,
    },
    SetSaturation {
        saturation: u16,
    },
    SetBrightness {
        brightness: u16,
    },
    SetKelvin {
        kelvin: u16,
    },
}

impl UserAction {
    pub fn execute(&self, lifx_manager: LifxManager, device: DeviceInfo) {
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
        }
    }

    pub fn variants() -> &'static [UserAction] {
        &[
            UserAction::Refresh,
            UserAction::TogglePower,
            UserAction::SetPower { power: true },
            UserAction::SetHue { hue: 0 },
            UserAction::SetSaturation { saturation: 0 },
            UserAction::SetBrightness { brightness: 0 },
            UserAction::SetKelvin { kelvin: 3500 },
            UserAction::SetColor {
                hue: 0,
                saturation: 0,
                brightness: 0,
                kelvin: 3500,
            },
        ]
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, device: Option<DeviceInfo>) -> egui::Response {
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
            UserAction::SetPower { power } => ui.checkbox(power, "Power"),
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

    use super::*;
    use crate::{device_info::GroupInfo, LifxManager};

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
            kelvin: 3500,
        };
        assert_eq!(
            format!("{}", action),
            "Set Color: H: 120, S: 8, B: 6, K: 3500"
        );
    }

    #[test]
    fn test_user_action_variants() {
        let variants = UserAction::variants();
        assert!(variants.contains(&UserAction::Refresh));
        assert!(variants.contains(&UserAction::TogglePower));
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
}
