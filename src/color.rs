use eframe::egui::{Color32, Rgba};
use lifx_core::HSBK;
use serde::{Deserialize, Serialize};
use strum_macros::{EnumIter, VariantNames};

pub const DEFAULT_KELVIN: u16 = 3500;

/// Enumerate each field of HSBK, the color space used by LIFX bulbs.
/// HSBK stands for Hue, Saturation, Brightness, Kelvin.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Serialize,
    Deserialize,
    EnumIter,
    VariantNames,
    strum_macros::AsRefStr,
)]
pub enum HSBKField {
    Hue,
    Saturation,
    Brightness,
    Kelvin,
}

/// Standard color representation for most monitors. Keeps track
/// of the temperature of the color too for conversion convenience.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RGB8 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub temperature: Option<u16>,
}

impl RGB8 {
    pub fn new(red: u8, green: u8, blue: u8, temperature: Option<u16>) -> RGB8 {
        RGB8 {
            red,
            green,
            blue,
            temperature,
        }
    }

    pub fn iter(&self) -> std::array::IntoIter<u8, 3> {
        [self.red, self.green, self.blue].into_iter()
    }
}

impl From<HSBK> for RGB8 {
    fn from(hsbk: HSBK) -> RGB8 {
        let HSBK {
            hue,
            saturation,
            brightness,
            kelvin,
        } = hsbk;
        let saturation_ratio = (100.0 * saturation as f64 / u16::MAX as f64) / 100.0;
        let brightness_ratio = (100.0 * brightness as f64 / u16::MAX as f64) / 100.0;
        let chroma = brightness_ratio * saturation_ratio;
        let hue_prime = (360.0 * hue as f64 / u16::MAX as f64) / 60.0;
        let mut temp = hue_prime;

        while temp >= 2.0 {
            temp -= 2.0;
        }

        let second_component = chroma * (1.0 - (temp - 1.0).abs());

        let (red, green, blue) = match hue_prime.floor() as i32 {
            0 => (chroma, second_component, 0.0),
            1 => (second_component, chroma, 0.0),
            2 => (0.0, chroma, second_component),
            3 => (0.0, second_component, chroma),
            4 => (second_component, 0.0, chroma),
            5 => (chroma, 0.0, second_component),
            _ => (0.0, 0.0, 0.0),
        };

        let match_value = brightness_ratio - chroma;
        let (red, green, blue) = (red + match_value, green + match_value, blue + match_value);

        let rgb_hsb = RGB8 {
            red: (red * 255.0) as u8,
            green: (green * 255.0) as u8,
            blue: (blue * 255.0) as u8,
            temperature: None,
        };

        let rgb_k = kelvin_to_rgb(kelvin);
        let normalized_saturation = saturation as f64 / u16::MAX as f64;
        let normalized_brightness = (1.0 - normalized_saturation) / 255.0;

        let red = (rgb_hsb.red as f64
            * (normalized_saturation + rgb_k.red as f64 * normalized_brightness))
            .round() as u8;
        let green = (rgb_hsb.green as f64
            * (normalized_saturation + rgb_k.green as f64 * normalized_brightness))
            .round() as u8;
        let blue = (rgb_hsb.blue as f64
            * (normalized_saturation + rgb_k.blue as f64 * normalized_brightness))
            .round() as u8;

        RGB8 {
            red,
            green,
            blue,
            temperature: Some(hsbk.kelvin),
        }
    }
}

impl From<RGB8> for HSBK {
    fn from(color: RGB8) -> HSBK {
        let max_color_component = *[color.red, color.green, color.blue]
            .iter()
            .max()
            .expect("Invalid color tuple") as f32;
        let min_color_component = *[color.red, color.green, color.blue]
            .iter()
            .min()
            .expect("Invalid color tuple") as f32;
        let color_range = max_color_component - min_color_component;

        let brightness = ((max_color_component / 255.0) * u16::MAX as f32) as u16;

        let (saturation, hue) = if color_range != 0.0 {
            let saturation = ((color_range / max_color_component) * u16::MAX as f32) as u16;

            let redc = (max_color_component - color.red as f32) / color_range;
            let greenc = (max_color_component - color.green as f32) / color_range;
            let bluec = (max_color_component - color.blue as f32) / color_range;

            let mut hue = if color.red as f32 == max_color_component {
                bluec - greenc
            } else if color.green as f32 == max_color_component {
                2.0 + redc - bluec
            } else {
                4.0 + greenc - redc
            };

            hue /= 6.0;
            if hue < 0.0 {
                hue += 1.0;
            }

            (saturation, (hue * u16::MAX as f32) as u16)
        } else {
            (0, 0)
        };

        HSBK {
            hue,
            saturation,
            brightness,
            kelvin: color.temperature.unwrap_or(DEFAULT_KELVIN),
        }
    }
}

impl From<RGB8> for Color32 {
    fn from(rgb: RGB8) -> Color32 {
        Color32::from_rgb(rgb.red, rgb.green, rgb.blue)
    }
}

/// Convert a scalar Kelvin temperature to an RGB color.
pub fn kelvin_to_rgb(temperature: u16) -> RGB8 {
    let percentage_temperature = temperature / 100;
    let red;
    let green;

    if percentage_temperature <= 66 {
        red = 255.;
        green = (99.4708025861 * (percentage_temperature as f64 + 0.0000000001).ln()
            - 161.1195681661)
            .clamp(0.0, 255.0);
    } else {
        red = 329.698727466
            * ((percentage_temperature - 60) as f64)
                .powf(-0.1332047592)
                .clamp(0.0, 255.0);
        green = (288.1221695283 * ((percentage_temperature - 60) as f64).powf(-0.0755148492))
            .clamp(0.0, 255.0);
    }

    let blue = if percentage_temperature >= 66 {
        255.0
    } else if percentage_temperature <= 19 {
        0.0
    } else {
        (138.5177312231 * ((percentage_temperature - 10) as f64).ln() - 305.0447927307)
            .clamp(0.0, 255.0)
    };

    RGB8 {
        red: red as u8,
        green: green as u8,
        blue: blue as u8,
        temperature: Some(temperature),
    }
}

pub fn default_hsbk() -> HSBK {
    HSBK {
        hue: 0,
        saturation: 0,
        brightness: 0,
        kelvin: 0,
    }
}

/// Used for preventing overflow when working with HSBK values
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct HSBK32 {
    pub hue: u32,
    pub saturation: u32,
    pub brightness: u32,
    pub kelvin: u32,
}

impl From<HSBK> for HSBK32 {
    fn from(hsbk: HSBK) -> HSBK32 {
        HSBK32 {
            hue: hsbk.hue as u32,
            saturation: hsbk.saturation as u32,
            brightness: hsbk.brightness as u32,
            kelvin: hsbk.kelvin as u32,
        }
    }
}

impl From<HSBK32> for HSBK {
    fn from(hsbk: HSBK32) -> HSBK {
        HSBK {
            hue: hsbk.hue as u16,
            saturation: hsbk.saturation as u16,
            brightness: hsbk.brightness as u16,
            kelvin: hsbk.kelvin as u16,
        }
    }
}

impl From<RGB8> for HSBK32 {
    fn from(rgb: RGB8) -> HSBK32 {
        let hsbk: HSBK = rgb.into();
        hsbk.into()
    }
}

impl From<HSBK32> for RGB8 {
    fn from(hsbk: HSBK32) -> RGB8 {
        let hsbk: HSBK = hsbk.into();
        hsbk.into()
    }
}

impl From<Color32> for HSBK32 {
    fn from(color: Color32) -> HSBK32 {
        let rgb = RGB8 {
            red: color.r(),
            green: color.g(),
            blue: color.b(),
            temperature: None,
        };

        let hsbk: HSBK = rgb.into();
        hsbk.into()
    }
}

impl From<HSBK32> for Color32 {
    fn from(hsbk: HSBK32) -> Color32 {
        let rgb: RGB8 = hsbk.into();
        rgb.into()
    }
}

/// Given an Rgba color, return the contrast color to use for text.
pub fn contrast_color(color: impl Into<Rgba>) -> Color32 {
    if color.into().intensity() < 0.5 {
        Color32::WHITE
    } else {
        Color32::BLACK
    }
}

/// A color that can be used to change the color of a light over time.
/// Many LIFX functions accept a `duration` parameter that specifies
/// how long the color change should take.
#[derive(Debug, Clone, Copy)]
pub struct DeltaColor {
    pub next: HSBK,
    pub duration: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kelvin_to_rgb() {
        let color = kelvin_to_rgb(DEFAULT_KELVIN);
        assert_eq!(color.red, 255);
        assert_eq!(color.green, 192);
        assert_eq!(color.blue, 140);
    }

    #[test]
    fn test_rgb_to_hsbk() {
        let color = RGB8 {
            red: 255,
            green: 191,
            blue: 0,
            temperature: Some(DEFAULT_KELVIN),
        };
        let hsbk: HSBK = color.into();
        assert_eq!(hsbk.hue, 8181);
        assert_eq!(hsbk.saturation, 65535);
        assert_eq!(hsbk.brightness, 65535);
        assert_eq!(hsbk.kelvin, DEFAULT_KELVIN);
    }

    #[test]
    fn test_hsbk_to_rgb() {
        let hsbk = HSBK {
            hue: 0,
            saturation: 65535,
            brightness: 65535,
            kelvin: DEFAULT_KELVIN,
        };
        let color: RGB8 = hsbk.into();
        assert_eq!(color.red, 255);
        assert_eq!(color.green, 0);
        assert_eq!(color.blue, 0);
        assert_eq!(color.temperature, Some(DEFAULT_KELVIN));
    }

    #[test]
    fn test_contrast_color() {
        let color = Color32::BLACK;
        assert_eq!(contrast_color(color), Color32::WHITE);

        let color = Color32::WHITE;
        assert_eq!(contrast_color(color), Color32::BLACK);
    }
}
