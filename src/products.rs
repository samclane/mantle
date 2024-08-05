use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::ops::RangeInclusive;

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
        if let Some((_, product)) = model {
            if let Some(info) = products.get(product) {
                info.features.clone()
            } else {
                Features::default()
            }
        } else {
            Features::default()
        }
    }

    pub fn as_ref(&self) -> Option<Features> {
        Some(self.clone())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Products {
    pub products: Vec<Product>,
}

pub fn get_products() -> HashMap<u32, Product> {
    let file = File::open("src/data/products.json").unwrap();
    let reader = BufReader::new(file);
    let products: Products = serde_json::from_reader(reader).unwrap();
    let mut product_map = HashMap::new();
    for product in products.products {
        product_map.insert(product.pid, product);
    }
    product_map
}
