use lifx_core::HSBK;

use crate::{color::default_hsbk, device_info::DeviceInfo, LifxManager};

pub struct Scene {
    pub device_color_pairs: Vec<(DeviceInfo, HSBK)>,
}

impl Scene {
    /// A scene defines a set of devices and their colors so that they can be applied all at once.
    /// This is useful for setting up a specific lighting configuration that you want to be able to
    /// apply quickly.

    pub fn new(device_color_pairs: Vec<(DeviceInfo, HSBK)>) -> Self {
        Self { device_color_pairs }
    }

    pub fn apply(&self, lifx_manager: &mut LifxManager) {
        for (device, color) in &self.device_color_pairs {
            match device {
                DeviceInfo::Bulb(bulb) => {
                    lifx_manager.set_color(&&**bulb, *color, None).unwrap();
                }
                DeviceInfo::Group(group) => {
                    lifx_manager
                        .set_group_color(group, *color, &lifx_manager.bulbs.lock().unwrap(), None)
                        .unwrap();
                }
            }
        }
    }
}

impl From<Vec<DeviceInfo>> for Scene {
    fn from(devices: Vec<DeviceInfo>) -> Self {
        let device_color_pairs = devices
            .into_iter()
            .map(|device| (device.clone(), *device.color().unwrap_or(&default_hsbk())))
            .collect();

        Self::new(device_color_pairs)
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
        let scene = Scene::new(vec![(
            DeviceInfo::Bulb(Box::new(bulb.clone())),
            default_hsbk(),
        )]);

        assert_eq!(scene.device_color_pairs.len(), 1);
        assert_eq!(
            scene.device_color_pairs[0].0,
            DeviceInfo::Bulb(Box::new(bulb))
        );
        assert_eq!(scene.device_color_pairs[0].1, default_hsbk());
    }
}
