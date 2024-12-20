use lifx_core::HSBK;
use serde::{Deserialize, Serialize};

use crate::{color::default_hsbk, device_info::DeviceInfo, LifxManager, HSBK32};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub device_color_pairs: Vec<(DeviceInfo, HSBK32)>,
    pub name: String,
}

impl Scene {
    /// A scene defines a set of devices and their colors so that they can be applied all at once.
    /// This is useful for setting up a specific lighting configuration that you want to be able to
    /// apply quickly.
    pub fn new(device_color_pairs: Vec<(DeviceInfo, HSBK32)>, name: String) -> Self {
        Self {
            device_color_pairs,
            name,
        }
    }

    pub fn apply(&self, lifx_manager: &mut LifxManager) {
        for (device, color) in &self.device_color_pairs {
            let color = HSBK::from(*color);
            match device {
                DeviceInfo::Bulb(bulb) => {
                    lifx_manager.set_color(&&**bulb, color, None).unwrap();
                }
                DeviceInfo::Group(group) => {
                    lifx_manager
                        .set_group_color(group, color, &lifx_manager.bulbs.lock().unwrap(), None)
                        .unwrap();
                }
            }
        }
    }

    pub fn devices(&self) -> Vec<DeviceInfo> {
        self.device_color_pairs
            .iter()
            .map(|(device_info, _)| device_info.clone())
            .collect()
    }

    pub fn devices_mut(&mut self) -> impl Iterator<Item = &mut DeviceInfo> {
        self.device_color_pairs
            .iter_mut()
            .map(|(device_info, _)| device_info)
    }
}

impl From<Vec<DeviceInfo>> for Scene {
    fn from(devices: Vec<DeviceInfo>) -> Self {
        let device_color_pairs = devices
            .into_iter()
            .map(|device| {
                let color: HSBK32 = (*device.color().unwrap_or(&default_hsbk())).into();
                (device.clone(), color)
            })
            .collect();

        Self::new(device_color_pairs, "Unnamed Scene".to_string())
    }
}

#[cfg(test)]
mod test {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use super::*;
    use crate::device_info::BulbInfo;

    #[test]
    fn test_scene_from_vec() {
        let source = 1234;
        let target = 5678;
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 56700);
        let mut bulb = BulbInfo::new(source, target, addr);
        bulb.update(addr);
        let scene = Scene::from(vec![DeviceInfo::Bulb(Box::new(bulb.clone()))]);

        assert_eq!(scene.device_color_pairs.len(), 1);
        assert_eq!(
            scene.device_color_pairs[0].0,
            DeviceInfo::Bulb(Box::new(bulb))
        );
    }

    #[test]
    fn test_create_scene() {
        let source = 1234;
        let target = 5678;
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 56700);
        let mut bulb = BulbInfo::new(source, target, addr);
        bulb.update(addr);
        let scene = Scene::new(
            vec![(DeviceInfo::Bulb(Box::new(bulb.clone())), HSBK32::default())],
            "Test Scene".to_string(),
        );

        assert_eq!(scene.device_color_pairs.len(), 1);
        assert_eq!(
            scene.device_color_pairs[0].0,
            DeviceInfo::Bulb(Box::new(bulb))
        );
        assert_eq!(scene.device_color_pairs[0].1, HSBK32::default());
    }
}
