use eframe::egui::{Color32, Rgba};
use lifx_core::HSBK;
use serde::{Deserialize, Serialize};

const DEFAULT_KELVIN: u16 = 3500;

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
        let a = saturation as f64 / u16::MAX as f64;
        let b = (1.0 - a) / 255.0;

        let red: u8 = (rgb_hsb.red as f64 * (a + rgb_k.red as f64 * b)).round() as u8;
        let green = (rgb_hsb.green as f64 * (a + rgb_k.green as f64 * b)).round() as u8;
        let blue = (rgb_hsb.blue as f64 * (a + rgb_k.blue as f64 * b)).round() as u8;

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
        let cmax = *[color.red, color.green, color.blue]
            .iter()
            .max()
            .expect("Invalid color tuple") as f32;
        let cmin = *[color.red, color.green, color.blue]
            .iter()
            .min()
            .expect("Invalid color tuple") as f32;
        let cdel = cmax - cmin;

        let brightness = ((cmax / 255.0) * u16::MAX as f32) as u16;

        let (saturation, hue) = if cdel != 0.0 {
            let saturation = ((cdel / cmax) * u16::MAX as f32) as u16;

            let redc = (cmax - color.red as f32) / cdel;
            let greenc = (cmax - color.green as f32) / cdel;
            let bluec = (cmax - color.blue as f32) / cdel;

            let mut hue = if color.red as f32 == cmax {
                bluec - greenc
            } else if color.green as f32 == cmax {
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

pub fn kelvin_to_rgb(temperature: u16) -> RGB8 {
    let p_temp = temperature / 100;
    let red;
    let green;

    if p_temp <= 66 {
        red = 255.;
        green = (99.4708025861 * (p_temp as f64 + 0.0000000001).ln() - 161.1195681661)
            .clamp(0.0, 255.0);
    } else {
        red = 329.698727466 * ((p_temp - 60) as f64).powf(-0.1332047592).clamp(0.0, 255.0);
        green = (288.1221695283 * ((p_temp - 60) as f64).powf(-0.0755148492)).clamp(0.0, 255.0);
    }

    let blue = if p_temp >= 66 {
        255.0
    } else if p_temp <= 19 {
        0.0
    } else {
        (138.5177312231 * ((p_temp - 10) as f64).ln() - 305.0447927307).clamp(0.0, 255.0)
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

// Used for preventing overflow when working with HSBK values
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

pub fn contrast_color(color: impl Into<Rgba>) -> Color32 {
    if color.into().intensity() < 0.5 {
        Color32::WHITE
    } else {
        Color32::BLACK
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DeltaColor {
    pub next: HSBK,
    pub duration: Option<u32>,
}
