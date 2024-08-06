// From https://github.com/samclane/LIFX-Control-Panel/blob/master/lifx_control_panel/utilities/utils.py
use lifx_core::HSBK;

pub fn hsbk_to_rgb(hsbk: HSBK) -> (u8, u8, u8) {
    let HSBK {
        hue,
        saturation,
        brightness,
        kelvin,
    } = hsbk;
    let saturation_ratio = (100.0 * saturation as f64 / 65535.0) / 100.0;
    let brightness_ratio = (100.0 * brightness as f64 / 65535.0) / 100.0;
    let chroma = brightness_ratio * saturation_ratio;
    let hue_prime = (360.0 * hue as f64 / 65535.0) / 60.0;
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
    let a = saturation as f64 / 65535.0;
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
