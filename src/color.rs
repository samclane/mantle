use eframe::egui::Color32;
use lifx_core::HSBK;

const DEFAULT_KELVIN: u16 = 3500;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RGB {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub temperature: Option<u16>,
}

impl RGB {
    pub fn new(red: u8, green: u8, blue: u8, temperature: Option<u16>) -> RGB {
        RGB {
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

impl From<HSBK> for RGB {
    fn from(hsbk: HSBK) -> RGB {
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

        let rgb_hsb = RGB {
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

        RGB {
            red,
            green,
            blue,
            temperature: Some(hsbk.kelvin),
        }
    }
}

impl From<RGB> for HSBK {
    fn from(color: RGB) -> HSBK {
        let cmax = *[color.red, color.green, color.blue].iter().max().unwrap() as f32;
        let cmin = *[color.red, color.green, color.blue].iter().min().unwrap() as f32;
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

impl From<RGB> for Color32 {
    fn from(rgb: RGB) -> Color32 {
        Color32::from_rgb(rgb.red, rgb.green, rgb.blue)
    }
}

pub fn kelvin_to_rgb(temperature: u16) -> RGB {
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

    RGB {
        red: red as u8,
        green: green as u8,
        blue: blue as u8,
        temperature: Some(temperature),
    }
}
