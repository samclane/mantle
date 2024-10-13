use serde::{Deserialize, Serialize};
use std::fmt::Display;

use crate::LifxManager;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum UserAction {
    Refresh,
    TogglePower,
    SetColor {
        hue: f32,
        saturation: f32,
        brightness: f32,
        kelvin: u16,
    },
    SetPower {
        power: bool,
    },
    SetBrightness {
        brightness: f32,
    },
    SetSaturation {
        saturation: f32,
    },
    SetKelvin {
        kelvin: u16,
    },
    SetHue {
        hue: f32,
    },
}

impl UserAction {
    pub fn execute(&self, lifx_manager: LifxManager) {
        match self {
            UserAction::Refresh => {
                lifx_manager.refresh();
            }
            UserAction::TogglePower => {
                lifx_manager.toggle_power();
            }
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
                // Implement color setting logic
            }
            UserAction::SetPower { power } => {
                log::info!("Executing action: Set Power - {}", power);
                // Implement power setting logic
            }
            UserAction::SetBrightness { brightness } => {
                log::info!("Executing action: Set Brightness - {}", brightness);
                // Implement brightness setting logic
            }
            UserAction::SetSaturation { saturation } => {
                log::info!("Executing action: Set Saturation - {}", saturation);
                // Implement saturation setting logic
            }
            UserAction::SetKelvin { kelvin } => {
                log::info!("Executing action: Set Kelvin - {}", kelvin);
                // Implement kelvin setting logic
            }
            UserAction::SetHue { hue } => {
                log::info!("Executing action: Set Hue - {}", hue);
                // Implement hue setting logic
            }
        }
    }

    pub fn variants() -> &'static [UserAction] {
        &[
            UserAction::Refresh,
            UserAction::TogglePower,
            UserAction::SetPower { power: true },
            UserAction::SetBrightness { brightness: 1.0 },
            UserAction::SetSaturation { saturation: 1.0 },
            UserAction::SetKelvin { kelvin: 3500 },
            UserAction::SetHue { hue: 0.0 },
            UserAction::SetColor {
                hue: 0.0,
                saturation: 1.0,
                brightness: 1.0,
                kelvin: 3500,
            },
        ]
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
    use super::*;
    use crate::LifxManager;

    #[test]
    fn test_user_action_display() {
        let action = UserAction::Refresh;
        assert_eq!(format!("{}", action), "Refresh");

        let action = UserAction::SetBrightness { brightness: 0.5 };
        assert_eq!(format!("{}", action), "Set Brightness: 0.5");

        let action = UserAction::SetColor {
            hue: 120.0,
            saturation: 0.8,
            brightness: 0.6,
            kelvin: 3500,
        };
        assert_eq!(
            format!("{}", action),
            "Set Color: H: 120, S: 0.8, B: 0.6, K: 3500"
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
        action.execute(manager.clone());

        let action = UserAction::SetBrightness { brightness: 0.5 };
        action.execute(manager.clone());
    }
}
