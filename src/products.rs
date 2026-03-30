use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::RangeInclusive;

static PRODUCTS: &str = include_str!("../data/products.json");

pub const LIFX_RANGE: RangeInclusive<u16> = 0..=u16::MAX;
pub const KELVIN_RANGE: TemperatureRange = TemperatureRange {
    min: 2500,
    max: 9000,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Upgrade {
    pub major: u32,
    pub minor: u32,
    pub features: Features,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Product {
    pub pid: u32,
    pub name: String,
    pub features: Features,
    pub upgrades: Vec<Upgrade>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemperatureRange {
    pub min: u32,
    pub max: u32,
}

impl TemperatureRange {
    pub fn to_range_u16(&self) -> RangeInclusive<u16> {
        self.min as u16..=self.max as u16
    }

    pub fn to_range_f32(&self) -> RangeInclusive<f32> {
        self.min as f32..=self.max as f32
    }
}

impl From<TemperatureRange> for (u16, u16) {
    fn from(range: TemperatureRange) -> Self {
        (range.min as u16, range.max as u16)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Features {
    pub color: Option<bool>,
    pub chain: Option<bool>,
    pub matrix: Option<bool>,
    pub infrared: Option<bool>,
    pub multizone: Option<bool>,
    pub temperature_range: Option<TemperatureRange>,
    pub hev: Option<bool>,
    pub relays: Option<bool>,
    pub buttons: Option<bool>,
}

impl Features {
    pub fn get_features(model: Option<&(u32, u32)>) -> Features {
        let products = get_products();
        model
            .and_then(|(_, product)| products.get(product))
            .map(|info| info.features.clone())
            .unwrap_or_default()
    }

    pub fn as_ref(&self) -> Option<Features> {
        Some(self.clone())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Products {
    pub products: Vec<Product>,
}

pub fn get_product_name(model: Option<&(u32, u32)>) -> Option<String> {
    let products = get_products();
    model
        .and_then(|(_, product)| products.get(product))
        .map(|info| info.name.clone())
}

pub fn get_products() -> HashMap<u32, Product> {
    let products: Products = serde_json::from_str(PRODUCTS).expect("Failed to parse products json");
    let mut product_map = HashMap::new();
    for product in products.products {
        product_map.insert(product.pid, product);
    }
    product_map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_products_parses_embedded_json() {
        let products = get_products();
        assert!(!products.is_empty());
    }

    #[test]
    fn get_products_contains_known_pid() {
        let products = get_products();
        let product = products.get(&1).expect("PID 1 should exist");
        assert_eq!(product.name, "LIFX Original 1000");
    }

    #[test]
    fn get_product_name_valid_model() {
        let model = (1u32, 1u32);
        let name = get_product_name(Some(&model));
        assert_eq!(name, Some("LIFX Original 1000".to_string()));
    }

    #[test]
    fn get_product_name_invalid_model() {
        let model = (0u32, 999999u32);
        assert_eq!(get_product_name(Some(&model)), None);
    }

    #[test]
    fn get_product_name_none_input() {
        assert_eq!(get_product_name(None), None);
    }

    #[test]
    fn temperature_range_to_range_u16() {
        let range = TemperatureRange {
            min: 2500,
            max: 9000,
        };
        assert_eq!(range.to_range_u16(), 2500u16..=9000u16);
    }

    #[test]
    fn temperature_range_to_range_f32() {
        let range = TemperatureRange {
            min: 2500,
            max: 9000,
        };
        assert_eq!(range.to_range_f32(), 2500.0f32..=9000.0f32);
    }

    #[test]
    fn temperature_range_into_tuple() {
        let range = TemperatureRange {
            min: 2500,
            max: 9000,
        };
        let tuple: (u16, u16) = range.into();
        assert_eq!(tuple, (2500, 9000));
    }

    #[test]
    fn features_get_features_unknown_model_returns_default() {
        let features = Features::get_features(Some(&(0, 999999)));
        assert_eq!(features.color, None);
        assert_eq!(features.chain, None);
    }

    #[test]
    fn features_get_features_none_returns_default() {
        let features = Features::get_features(None);
        assert_eq!(features.color, None);
    }

    #[test]
    fn features_as_ref_returns_some() {
        let features = Features::default();
        assert!(features.as_ref().is_some());
    }

    #[test]
    fn kelvin_range_constants() {
        assert_eq!(KELVIN_RANGE.min, 2500);
        assert_eq!(KELVIN_RANGE.max, 9000);
    }

    #[test]
    fn lifx_range_full_u16() {
        assert_eq!(*LIFX_RANGE.start(), 0);
        assert_eq!(*LIFX_RANGE.end(), u16::MAX);
    }
}
