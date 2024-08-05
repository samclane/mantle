use crate::products::Features;
use crate::refreshable_data::RefreshableData;
use lifx_core::{get_product_info, BuildOptions, Message, RawMessage, HSBK};
use std::ffi::CString;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

const HOUR: Duration = Duration::from_secs(60 * 60);

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
    pub color: Color,
    pub features: Features,
}

#[derive(Debug)]
pub enum Color {
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
            color: Color::Unknown,
            features: Features::default(),
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
        match &self.color {
            Color::Unknown => (), // we'll need to wait to get info about this bulb's model, so we'll know if it's multizone or not
            Color::Single(d) => self.refresh_if_needed(sock, d)?,
            Color::Multi(d) => self.refresh_if_needed(sock, d)?,
        }
        self.features = Features::get_features(self.model.as_ref());

        Ok(())
    }

    pub fn get_color(&self) -> Option<&HSBK> {
        match self.color {
            Color::Single(ref data) => data.as_ref(),
            _ => None,
        }
    }
}

impl std::fmt::Debug for BulbInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
                    Color::Unknown => write!(f, "??")?,
                    Color::Single(ref color) => {
                        f.write_str(
                            &color
                                .as_ref()
                                .map(|c| c.describe(false))
                                .unwrap_or_else(|| "??".to_owned()),
                        )?;
                    }
                    Color::Multi(ref color) => {
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
