// From https://github.com/samclane/LIFX-Control-Panel/blob/master/lifx_control_panel/utilities/utils.py
use lifx_core::HSBK;

const DEFAULT_KELVIN: u16 = 3500;

// RGB struct that's iterable
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

// HSBK conversion
impl From<HSBK> for RGB {
    fn from(hsbk: HSBK) -> RGB {
        let (red, green, blue) = hsbk_to_rgb(hsbk);
        RGB {
            red,
            green,
            blue,
            temperature: Some(hsbk.kelvin),
        }
    }
}

// RGB conversion
impl From<RGB> for HSBK {
    fn from(rgb: RGB) -> HSBK {
        let (hue, saturation, brightness, kelvin) = rgb_to_hsbk(rgb);
        HSBK {
            hue,
            saturation,
            brightness,
            kelvin,
        }
    }
}

pub fn hsbk_to_rgb(hsbk: HSBK) -> (u8, u8, u8) {
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

    let rgb_hsb = (
        (red * 255.0) as u8,
        (green * 255.0) as u8,
        (blue * 255.0) as u8,
    );

    let rgb_k = kelvin_to_rgb(kelvin);
    let a = saturation as f64 / u16::MAX as f64;
    let b = (1.0 - a) / 255.0;

    let x = (rgb_hsb.0 as f64 * (a + rgb_k.0 as f64 * b)).round() as u8;
    let y = (rgb_hsb.1 as f64 * (a + rgb_k.1 as f64 * b)).round() as u8;
    let z = (rgb_hsb.2 as f64 * (a + rgb_k.2 as f64 * b)).round() as u8;

    (x, y, z)
}

pub fn kelvin_to_rgb(temperature: u16) -> (u8, u8, u8) {
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

    (red as u8, green as u8, blue as u8)
}

pub fn rgb_to_hsbk(color: RGB) -> (u16, u16, u16, u16) {
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

    (
        hue,
        saturation,
        brightness,
        color.temperature.unwrap_or(DEFAULT_KELVIN),
    )
}
