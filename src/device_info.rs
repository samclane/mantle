use crate::products::Features;
use crate::refreshable_data::RefreshableData;
use lifx_core::{get_product_info, BuildOptions, LifxIdent, LifxString, Message, RawMessage, HSBK};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::CString;
use std::fmt::{Display, Formatter};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, UdpSocket};
use std::time::{Duration, Instant, SystemTime};

const HOUR: Duration = Duration::from_secs(60 * 60);

#[derive(Clone)]
pub struct GroupInfo {
    pub group: LifxIdent,
    pub label: LifxString,
    pub updated_at: u64,
}

impl PartialEq for GroupInfo {
    fn eq(&self, other: &Self) -> bool {
        self.group == other.group
    }
}

pub struct BulbInfo {
    pub last_seen: Instant,
    pub source: u32,
    pub target: u64,
    pub addr: SocketAddr,
    pub name: RefreshableData<CString>,
    pub model: RefreshableData<(u32, u32)>,
    pub location: RefreshableData<CString>,
    pub host_firmware: RefreshableData<(u16, u16)>,
    pub wifi_firmware: RefreshableData<(u16, u16)>,
    pub power_level: RefreshableData<u16>,
    pub color: DeviceColor,
    pub features: Features,
    pub group: RefreshableData<GroupInfo>,
}

impl Clone for BulbInfo {
    fn clone(&self) -> Self {
        BulbInfo {
            last_seen: self.last_seen,
            source: self.source,
            target: self.target,
            addr: self.addr,
            name: self.name.clone(),
            model: self.model.clone(),
            location: self.location.clone(),
            host_firmware: self.host_firmware.clone(),
            wifi_firmware: self.wifi_firmware.clone(),
            power_level: self.power_level.clone(),
            color: self.color.clone(),
            features: self.features.clone(),
            group: self.group.clone(),
        }
    }
}

impl PartialEq for BulbInfo {
    fn eq(&self, other: &Self) -> bool {
        self.target == other.target
    }
}

#[derive(Clone)]
pub enum DeviceInfo {
    Bulb(Box<BulbInfo>),
    Group(GroupInfo),
}

impl DeviceInfo {
    pub fn id(&self) -> u64 {
        match self {
            DeviceInfo::Bulb(b) => b.target,
            DeviceInfo::Group(g) => u64::from_le_bytes(
                g.group.0[0..8]
                    .try_into()
                    .expect("Failed to convert ident to u64"),
            ),
        }
    }

    pub fn name(&self) -> Option<String> {
        match self {
            DeviceInfo::Bulb(b) => b
                .name
                .data
                .as_ref()
                .map(|n| n.to_string_lossy().to_string()),
            DeviceInfo::Group(g) => Some(g.label.to_string()),
        }
    }
}

impl Serialize for DeviceInfo {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (self.id(), self.name()).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DeviceInfo {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (id, name) = <(u64, Option<String>)>::deserialize(deserializer)?;
        Ok(DeviceInfo::Bulb(Box::new(BulbInfo {
            last_seen: Instant::now(),
            source: 0,
            target: id,
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 56700),
            name: RefreshableData::new(
                CString::new(name.unwrap_or_default()).expect("Failed to create CString"),
                HOUR,
                Message::GetLabel,
            ),
            model: RefreshableData::empty(HOUR, Message::GetVersion),
            location: RefreshableData::empty(HOUR, Message::GetLocation),
            host_firmware: RefreshableData::empty(HOUR, Message::GetHostFirmware),
            wifi_firmware: RefreshableData::empty(HOUR, Message::GetWifiFirmware),
            power_level: RefreshableData::empty(Duration::from_secs(15), Message::GetPower),
            color: DeviceColor::Unknown,
            features: Features::default(),
            group: RefreshableData::empty(Duration::from_secs(15), Message::GetGroup),
        })))
    }
}

impl Display for DeviceInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceInfo::Bulb(b) => write!(
                f,
                "{}",
                b.name.data.as_ref().map_or_else(
                    || "Unknown".to_string(),
                    |name| name.to_string_lossy().to_string()
                )
            ),
            DeviceInfo::Group(g) => write!(f, "{}", g.label.cstr().to_str().unwrap_or_default()),
        }
    }
}

impl PartialEq for DeviceInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

#[derive(Debug, Clone)]
pub enum DeviceColor {
    Unknown,
    Single(RefreshableData<HSBK>),
    Multi(RefreshableData<Vec<Option<HSBK>>>),
}

impl BulbInfo {
    pub fn new(source: u32, target: u64, addr: SocketAddr) -> BulbInfo {
        BulbInfo {
            last_seen: Instant::now(),
            source,
            target,
            addr,
            name: RefreshableData::empty(HOUR, Message::GetLabel),
            model: RefreshableData::empty(HOUR, Message::GetVersion),
            location: RefreshableData::empty(HOUR, Message::GetLocation),
            host_firmware: RefreshableData::empty(HOUR, Message::GetHostFirmware),
            wifi_firmware: RefreshableData::empty(HOUR, Message::GetWifiFirmware),
            power_level: RefreshableData::empty(Duration::from_secs(15), Message::GetPower),
            color: DeviceColor::Unknown,
            features: Features::default(),
            group: RefreshableData::empty(Duration::from_secs(15), Message::GetGroup),
        }
    }

    pub fn update(&mut self, addr: SocketAddr) {
        self.last_seen = Instant::now();
        self.addr = addr;
    }

    fn refresh_if_needed<T>(
        &self,
        sock: &UdpSocket,
        data: &RefreshableData<T>,
    ) -> Result<(), failure::Error> {
        if data.needs_refresh() {
            let options = BuildOptions {
                target: Some(self.target),
                res_required: true,
                source: self.source,
                ..Default::default()
            };
            let message = RawMessage::build(&options, data.refresh_msg.clone())?;
            sock.send_to(&message.pack()?, self.addr)?;
        }
        Ok(())
    }

    pub fn query_for_missing_info(&mut self, sock: &UdpSocket) -> Result<(), failure::Error> {
        self.refresh_if_needed(sock, &self.name)?;
        self.refresh_if_needed(sock, &self.model)?;
        self.refresh_if_needed(sock, &self.location)?;
        self.refresh_if_needed(sock, &self.host_firmware)?;
        self.refresh_if_needed(sock, &self.wifi_firmware)?;
        self.refresh_if_needed(sock, &self.power_level)?;
        self.refresh_if_needed(sock, &self.group)?;
        match &self.color {
            DeviceColor::Unknown => (), // We'll need to wait to get info about this bulb's model.
            DeviceColor::Single(d) => self.refresh_if_needed(sock, d)?,
            DeviceColor::Multi(d) => self.refresh_if_needed(sock, d)?,
        }
        self.features = Features::get_features(self.model.as_ref());
        Ok(())
    }

    pub fn get_color(&self) -> Option<&HSBK> {
        match self.color {
            DeviceColor::Single(ref data) => data.as_ref(),
            DeviceColor::Multi(ref data) => handle_multizone(data.as_ref()),
            _ => None,
        }
    }

    pub fn group_label(&self) -> Option<String> {
        self.group
            .data
            .as_ref()
            .map(|g| &g.label)
            .map(|l| l.to_string())
    }

    pub fn name_label(&self) -> Option<String> {
        self.name
            .data
            .as_ref()
            .map(|n| n.to_string_lossy().to_string())
    }
}

fn handle_multizone(data: Option<&Vec<Option<HSBK>>>) -> Option<&HSBK> {
    data.and_then(|vec| vec.first())
        .and_then(|opt| opt.as_ref())
}

impl GroupInfo {
    pub fn new(group: LifxIdent, label: LifxString) -> GroupInfo {
        GroupInfo {
            group,
            label,
            updated_at: 0,
        }
    }

    pub fn build_all_group() -> GroupInfo {
        GroupInfo::new(
            LifxIdent([0u8; 16]),
            LifxString::new(&CString::new("All").expect("Failed to create CString")),
        )
    }

    pub fn update(&mut self) {
        // update time as current datetime
        self.updated_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("Failed to get time since epoch")
            .as_secs();
    }

    pub fn get_bulbs<'a>(&self, bulbs: &'a HashMap<u64, BulbInfo>) -> Vec<&'a BulbInfo> {
        if self.group == LifxIdent([0u8; 16]) {
            return bulbs.values().collect();
        }
        bulbs
            .values()
            .filter(|b| {
                b.group
                    .data
                    .as_ref()
                    .map_or(false, |g: &GroupInfo| g.group == self.group)
            })
            .collect()
    }

    pub fn any_on(&self, bulbs: &HashMap<u64, BulbInfo>) -> bool {
        self.get_bulbs(bulbs)
            .iter()
            .any(|b| b.power_level.data.unwrap_or(0) > 0)
    }

    pub fn id(&self) -> u64 {
        // Convert ident to u64
        u64::from_le_bytes(self.group.0[0..8].try_into().unwrap_or([0u8; 8]))
    }
}

impl std::fmt::Debug for BulbInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "BulbInfo({:0>16X} - {}  ", self.target, self.addr)?;

        if let Some(name) = self.name.as_ref() {
            write!(f, "{}", name.to_string_lossy())?;
        }
        if let Some(location) = self.location.as_ref() {
            write!(f, "/{}", location.to_string_lossy())?;
        }
        if let Some((vendor, product)) = self.model.as_ref() {
            if let Some(info) = get_product_info(*vendor, *product) {
                write!(f, " - {} ", info.name)?;
            } else {
                write!(
                    f,
                    " - Unknown model (vendor={}, product={}) ",
                    vendor, product
                )?;
            }
        }
        if let Some((major, minor)) = self.host_firmware.as_ref() {
            write!(f, " McuFW:{}.{}", major, minor)?;
        }
        if let Some((major, minor)) = self.wifi_firmware.as_ref() {
            write!(f, " WifiFW:{}.{}", major, minor)?;
        }
        if let Some(level) = self.power_level.as_ref() {
            if *level > 0 {
                write!(f, "  Powered On(")?;
                match self.color {
                    DeviceColor::Unknown => write!(f, "??")?,
                    DeviceColor::Single(ref color) => {
                        f.write_str(
                            &color
                                .as_ref()
                                .map(|c| c.describe(false))
                                .unwrap_or_else(|| "??".to_owned()),
                        )?;
                    }
                    DeviceColor::Multi(ref color) => {
                        if let Some(vec) = color.as_ref() {
                            write!(f, "Zones: ")?;
                            for zone in vec {
                                if let Some(color) = zone {
                                    write!(f, "{} ", color.describe(true))?;
                                } else {
                                    write!(f, "?? ")?;
                                }
                            }
                        }
                    }
                }
                write!(f, ")")?;
            } else {
                write!(f, "  Powered Off")?;
            }
        }
        if let Some(features) = self.features.as_ref() {
            write!(f, "  Features: {:?}", features)?;
        }
        write!(f, ")")
    }
}

impl std::fmt::Debug for GroupInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "GroupInfo({:?} - {}, {})",
            self.group, self.label, self.updated_at
        )
    }
}

impl std::fmt::Debug for DeviceInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DeviceInfo::Bulb(b) => write!(f, "{:?}", b),
            DeviceInfo::Group(g) => write!(f, "{:?}", g),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refreshable_data::RefreshableData;
    use lifx_core::{LifxIdent, LifxString, HSBK};
    use std::ffi::CString;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::u8;

    #[test]
    fn test_bulbinfo_new() {
        let source = 1234;
        let target = 5678;
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 56700);
        let bulb = BulbInfo::new(source, target, addr);

        assert_eq!(bulb.source, source);
        assert_eq!(bulb.target, target);
        assert_eq!(bulb.addr, addr);
        assert!(bulb.last_seen.elapsed() < Duration::from_secs(1));
        assert!(matches!(bulb.color, DeviceColor::Unknown));
        assert!(bulb.name.data.is_none());
        assert!(bulb.model.data.is_none());
        assert!(bulb.location.data.is_none());
    }

    #[test]
    fn test_bulbinfo_update() {
        let source = 1234;
        let target = 5678;
        let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 56700);
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 56701);
        let mut bulb = BulbInfo::new(source, target, addr1);

        std::thread::sleep(Duration::from_millis(10));
        bulb.update(addr2);

        assert_eq!(bulb.addr, addr2);
        assert!(bulb.last_seen.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn test_bulbinfo_get_color_single() {
        let source = 1234;
        let target = 5678;
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 56700);
        let mut bulb = BulbInfo::new(source, target, addr);

        let hsbk = HSBK {
            hue: 120,
            saturation: 65535,
            brightness: 32768,
            kelvin: 3500,
        };
        bulb.color = DeviceColor::Single(RefreshableData::new(
            hsbk,
            Duration::from_secs(60),
            Message::GetColorZones {
                start_index: 0,
                end_index: u8::MAX,
            },
        ));

        let color = bulb.get_color();
        assert!(color.is_some());
        assert_eq!(color.unwrap().hue, 120);
    }

    #[test]
    fn test_bulbinfo_get_color_multi() {
        let source = 1234;
        let target = 5678;
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 56700);
        let mut bulb = BulbInfo::new(source, target, addr);

        let hsbk1 = Some(HSBK {
            hue: 120,
            saturation: 65535,
            brightness: 32768,
            kelvin: 3500,
        });
        let hsbk2 = Some(HSBK {
            hue: 240,
            saturation: 65535,
            brightness: 32768,
            kelvin: 3500,
        });
        bulb.color = DeviceColor::Multi(RefreshableData::new(
            vec![hsbk1, hsbk2],
            Duration::from_secs(60),
            Message::GetColorZones {
                start_index: 0,
                end_index: u8::MAX,
            },
        ));

        let color = bulb.get_color();
        assert!(color.is_some());
        assert_eq!(color.unwrap().hue, 120);
    }

    #[test]
    fn test_groupinfo_new() {
        let ident = LifxIdent([1u8; 16]);
        let label = LifxString::new(&CString::new("TestGroup").unwrap());
        let group = GroupInfo::new(ident, label.clone());

        assert_eq!(group.group, ident);
        assert_eq!(group.label, label);
        assert_eq!(group.updated_at, 0);
    }

    #[test]
    fn test_groupinfo_build_all_group() {
        let group = GroupInfo::build_all_group();
        let expected_ident = LifxIdent([0u8; 16]);
        let expected_label = LifxString::new(&CString::new("All").unwrap());

        assert_eq!(group.group, expected_ident);
        assert_eq!(group.label, expected_label);
    }

    #[test]
    fn test_groupinfo_update() {
        let ident = LifxIdent([1u8; 16]);
        let label = LifxString::new(&CString::new("TestGroup").unwrap());
        let mut group = GroupInfo::new(ident, label);

        group.update();
        assert!(group.updated_at > 0);
    }

    #[test]
    fn test_groupinfo_get_bulbs() {
        let ident = LifxIdent([1u8; 16]);
        let label = LifxString::new(&CString::new("TestGroup").unwrap());
        let group = GroupInfo::new(ident.clone(), label);

        let bulb1 = BulbInfo::new(
            1,
            1,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 56700),
        );
        let bulb2 = BulbInfo::new(
            1,
            2,
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 56701),
        );

        let mut bulb_map = HashMap::new();

        let mut bulb1 = bulb1;
        bulb1.group.data = Some(group.clone());
        bulb_map.insert(bulb1.target, bulb1);

        bulb_map.insert(bulb2.target, bulb2);

        let bulbs = group.get_bulbs(&bulb_map);
        assert_eq!(bulbs.len(), 1);
        assert_eq!(bulbs[0].target, 1);
    }

    #[test]
    fn test_bulbinfo_name_label() {
        let source = 1234;
        let target = 5678;
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 56700);
        let mut bulb = BulbInfo::new(source, target, addr);

        let name = CString::new("TestBulb").unwrap();
        bulb.name.data = Some(name.clone());

        let label = bulb.name_label();
        assert!(label.is_some());
        assert_eq!(label.unwrap(), "TestBulb");
    }

    #[test]
    fn test_bulbinfo_group_label() {
        let source = 1234;
        let target = 5678;
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 56700);
        let mut bulb = BulbInfo::new(source, target, addr);

        let group_label = LifxString::new(&CString::new("TestGroup").unwrap());
        let group_info = GroupInfo::new(LifxIdent([1u8; 16]), group_label.clone());

        bulb.group.data = Some(group_info);

        let label = bulb.group_label();
        assert!(label.is_some());
        assert_eq!(label.unwrap(), "TestGroup");
    }

    #[test]
    fn test_deviceinfo_id() {
        let source = 1234;
        let target = 5678;
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 56700);
        let bulb = BulbInfo::new(source, target, addr);

        let device_info = DeviceInfo::Bulb(Box::new(bulb.clone()));
        assert_eq!(device_info.id(), target);

        let group_ident = LifxIdent([1u8; 16]);
        let group_label = LifxString::new(&CString::new("TestGroup").unwrap());
        let group = GroupInfo::new(group_ident, group_label);

        let device_info = DeviceInfo::Group(group.clone());
        let expected_id = u64::from_le_bytes(group_ident.0[0..8].try_into().unwrap());
        assert_eq!(device_info.id(), expected_id);
    }

    #[test]
    fn test_handle_multizone() {
        let hsbk1 = Some(HSBK {
            hue: 1000,
            saturation: 2000,
            brightness: 3000,
            kelvin: 4000,
        });
        let hsbk2 = Some(HSBK {
            hue: 5000,
            saturation: 6000,
            brightness: 7000,
            kelvin: 8000,
        });
        let data = Some(vec![hsbk1.clone(), hsbk2.clone()]);

        let color = handle_multizone(data.as_ref());
        assert!(color.is_some());
        assert_eq!(color.unwrap(), hsbk1.as_ref().unwrap());

        let empty_data: Option<&Vec<Option<HSBK>>> = None;
        let color = handle_multizone(empty_data);
        assert!(color.is_none());
    }
}
