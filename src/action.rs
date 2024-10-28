use eframe::egui;
use lifx_core::HSBK;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

use crate::{
    color::HSBKField,
    device_info::DeviceInfo,
    products::{KELVIN_RANGE, LIFX_RANGE},
    ui::hsbk_sliders,
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
    SetBrightness {
        brightness: u16,
    },
    SetSaturation {
        saturation: u16,
    },
    SetKelvin {
        kelvin: u16,
    },
    SetHue {
        hue: u16,
    },
}

impl UserAction {
    pub fn execute(&self, lifx_manager: LifxManager, device: DeviceInfo) {
        match self {
            UserAction::Refresh => {
                lifx_manager.refresh();
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
                    lifx_manager.set_power(&&*bulb_info, level).unwrap();
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
                        lifx_manager
                            .set_group_color(
                                &_group_info,
                                HSBK {
                                    hue: *hue,
                                    saturation: *saturation,
                                    brightness: *brightness,
                                    kelvin: *kelvin,
                                },
                                &lifx_manager.bulbs.lock().unwrap(),
                                None,
                            )
                            .unwrap();
                    }
                    DeviceInfo::Bulb(bulb_info) => {
                        lifx_manager
                            .set_color(
                                &&*bulb_info,
                                HSBK {
                                    hue: *hue,
                                    saturation: *saturation,
                                    brightness: *brightness,
                                    kelvin: *kelvin,
                                },
                                None,
                            )
                            .unwrap();
                    }
                }
            }
            UserAction::SetPower { power } => {
                log::info!("Executing action: Set Power - {}", power);
                match device {
                    DeviceInfo::Group(_group_info) => {
                        let power = if *power { u16::MAX } else { 0 };
                        lifx_manager
                            .set_group_power(
                                &_group_info,
                                &lifx_manager.bulbs.lock().unwrap(),
                                power,
                            )
                            .unwrap();
                    }
                    DeviceInfo::Bulb(bulb_info) => {
                        lifx_manager.set_power(&&*bulb_info, *power as u16).unwrap();
                    }
                }
            }
            UserAction::SetBrightness { brightness } => {
                log::info!("Executing action: Set Brightness - {}", brightness);
                match device {
                    DeviceInfo::Group(_group_info) => {
                        lifx_manager
                            .set_group_color_field(
                                &_group_info,
                                HSBKField::Brightness,
                                *brightness,
                                &lifx_manager.bulbs.lock().unwrap(),
                            )
                            .unwrap();
                    }
                    DeviceInfo::Bulb(bulb_info) => {
                        lifx_manager
                            .set_color_field(&&*bulb_info, HSBKField::Brightness, *brightness)
                            .unwrap();
                    }
                }
            }
            UserAction::SetSaturation { saturation } => {
                log::info!("Executing action: Set Saturation - {}", saturation);
                match device {
                    DeviceInfo::Group(_group_info) => {
                        lifx_manager
                            .set_group_color_field(
                                &_group_info,
                                HSBKField::Saturation,
                                *saturation,
                                &lifx_manager.bulbs.lock().unwrap(),
                            )
                            .unwrap();
                    }
                    DeviceInfo::Bulb(bulb_info) => {
                        lifx_manager
                            .set_color_field(&&*bulb_info, HSBKField::Saturation, *saturation)
                            .unwrap();
                    }
                }
            }
            UserAction::SetKelvin { kelvin } => {
                log::info!("Executing action: Set Kelvin - {}", kelvin);
                match device {
                    DeviceInfo::Group(_group_info) => {
                        lifx_manager
                            .set_group_color_field(
                                &_group_info,
                                HSBKField::Kelvin,
                                *kelvin,
                                &lifx_manager.bulbs.lock().unwrap(),
                            )
                            .unwrap();
                    }
                    DeviceInfo::Bulb(bulb_info) => {
                        lifx_manager
                            .set_color_field(&&*bulb_info, HSBKField::Kelvin, *kelvin)
                            .unwrap();
                    }
                }
            }
            UserAction::SetHue { hue } => {
                log::info!("Executing action: Set Hue - {}", hue);
                match device {
                    DeviceInfo::Group(_group_info) => {
                        lifx_manager
                            .set_group_color_field(
                                &_group_info,
                                HSBKField::Hue,
                                *hue,
                                &lifx_manager.bulbs.lock().unwrap(),
                            )
                            .unwrap();
                    }
                    DeviceInfo::Bulb(bulb_info) => {
                        lifx_manager
                            .set_color_field(&&*bulb_info, HSBKField::Hue, *hue)
                            .unwrap();
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
            UserAction::SetBrightness { brightness: 1 },
            UserAction::SetSaturation { saturation: 1 },
            UserAction::SetKelvin { kelvin: 3500 },
            UserAction::SetHue { hue: 0 },
            UserAction::SetColor {
                hue: 0,
                saturation: 1,
                brightness: 1,
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
                    hsbk_sliders(ui, hue, saturation, brightness, &device, kelvin)
                } else {
                    ui.label("No device selected")
                }
            }
            UserAction::SetPower { power } => ui.checkbox(power, "Power"),
            UserAction::SetBrightness { brightness } => {
                ui.add(egui::Slider::new(brightness, LIFX_RANGE).text("Brightness"))
            }
            UserAction::SetSaturation { saturation } => {
                ui.add(egui::Slider::new(saturation, LIFX_RANGE).text("Saturation"))
            }
            UserAction::SetKelvin { kelvin } => {
                ui.add(egui::Slider::new(kelvin, KELVIN_RANGE.to_range_u16()).text("Kelvin"))
            }
            UserAction::SetHue { hue } => ui.add(egui::Slider::new(hue, LIFX_RANGE).text("Hue")),
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
            UserAction::SetBrightness { brightness } => {
                write!(f, "Set Brightness: {}", brightness)
            }
            UserAction::SetSaturation { saturation } => {
                write!(f, "Set Saturation: {}", saturation)
            }
            UserAction::SetKelvin { kelvin } => write!(f, "Set Kelvin: {}", kelvin),
            UserAction::SetHue { hue } => write!(f, "Set Hue: {}", hue),
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
