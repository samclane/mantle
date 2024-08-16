use crate::bulb_info::{BulbInfo, GroupInfo};
use crate::color::{default_hsbk, HSBK32};
use crate::refreshable_data::RefreshableData;
use crate::Color;
use get_if_addrs::{get_if_addrs, IfAddr, Ifv4Addr};
use lifx_core::{get_product_info, BuildOptions, Message, RawMessage, Service, HSBK};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::spawn;
use std::time::{Duration, Instant};

pub struct Manager {
    pub bulbs: Arc<Mutex<HashMap<u64, BulbInfo>>>,
    pub last_discovery: Instant,
    pub sock: UdpSocket,
    pub source: u32,
}

impl Manager {
    pub fn new() -> Result<Manager, failure::Error> {
        let sock = UdpSocket::bind("0.0.0.0:56700")?;
        sock.set_broadcast(true)?;

        let recv_sock = sock.try_clone()?;

        let bulbs = Arc::new(Mutex::new(HashMap::new()));
        let receiver_bulbs = bulbs.clone();
        let source = 0x72757374;

        spawn(move || Self::worker(recv_sock, source, receiver_bulbs));

        let mut mgr = Manager {
            bulbs,
            last_discovery: Instant::now(),
            sock,
            source,
        };
        mgr.discover()?;
        Ok(mgr)
    }

    fn handle_message(raw: RawMessage, bulb: &mut BulbInfo) -> Result<(), lifx_core::Error> {
        match Message::from_raw(&raw)? {
            Message::StateService { port, service } => {
                if port != bulb.addr.port() as u32 || service != Service::UDP {
                    log::debug!("Unsupported service: {:?}/{}", service, port);
                }
            }
            Message::StateLabel { label } => bulb.name.update(label.cstr().to_owned()),
            Message::StateLocation { label, .. } => bulb.location.update(label.cstr().to_owned()),
            Message::StateVersion {
                vendor, product, ..
            } => {
                bulb.model.update((vendor, product));
                if let Some(info) = get_product_info(vendor, product) {
                    if info.multizone {
                        bulb.color = Color::Multi(RefreshableData::empty(
                            Duration::from_secs(15),
                            Message::GetColorZones {
                                start_index: 0,
                                end_index: 255,
                            },
                        ))
                    } else {
                        bulb.color = Color::Single(RefreshableData::empty(
                            Duration::from_secs(15),
                            Message::LightGet,
                        ))
                    }
                }
            }
            Message::StatePower { level } => bulb.power_level.update(level),
            Message::StateHostFirmware {
                version_minor,
                version_major,
                ..
            } => bulb.host_firmware.update((version_major, version_minor)),
            Message::StateWifiFirmware {
                version_minor,
                version_major,
                ..
            } => bulb.wifi_firmware.update((version_major, version_minor)),
            Message::LightState {
                color,
                power,
                label,
                ..
            } => {
                if let Color::Single(ref mut d) = bulb.color {
                    d.update(color);
                    bulb.power_level.update(power);
                }
                bulb.name.update(label.cstr().to_owned());
            }
            Message::StateZone {
                count,
                index,
                color,
            } => {
                if let Color::Multi(ref mut d) = bulb.color {
                    d.data.get_or_insert_with(|| {
                        let mut v = Vec::with_capacity(count as usize);
                        v.resize(count as usize, None);
                        assert!(index <= count);
                        v
                    })[index as usize] = Some(color);
                }
            }
            Message::StateMultiZone {
                count,
                index,
                color0,
                color1,
                color2,
                color3,
                color4,
                color5,
                color6,
                color7,
            } => {
                if let Color::Multi(ref mut d) = bulb.color {
                    let v = d.data.get_or_insert_with(|| {
                        let mut v = Vec::with_capacity(count as usize);
                        v.resize(count as usize, None);
                        assert!(index + 7 <= count);
                        v
                    });

                    // sometimes len(v) < index + 8 so we need to resize it
                    if v.len() < (index + 8) as usize {
                        v.resize((index + 8) as usize, None);
                    }
                    v[index as usize] = Some(color0);
                    v[index as usize + 1] = Some(color1);
                    v[index as usize + 2] = Some(color2);
                    v[index as usize + 3] = Some(color3);
                    v[index as usize + 4] = Some(color4);
                    v[index as usize + 5] = Some(color5);
                    v[index as usize + 6] = Some(color6);
                    v[index as usize + 7] = Some(color7);
                }
            }
            Message::Acknowledgement { seq } => {
                if raw.frame_addr.ack_required {
                    log::debug!("Received ack for sequence {}", seq);
                }
            }
            Message::LightStatePower { level } => {
                bulb.power_level.update(level);
            }
            Message::StateGroup {
                group,
                label,
                updated_at,
            } => {
                bulb.group.update(GroupInfo {
                    group,
                    label,
                    updated_at,
                });
            }
            unknown => {
                log::debug!("Received, but ignored {:?}", unknown);
            }
        }
        Ok(())
    }

    fn worker(
        recv_sock: UdpSocket,
        source: u32,
        receiver_bulbs: Arc<Mutex<HashMap<u64, BulbInfo>>>,
    ) {
        let mut buf = [0; 1024];
        loop {
            match recv_sock.recv_from(&mut buf) {
                Ok((0, addr)) => log::debug!("Received a zero-byte datagram from {:?}", addr),
                Ok((nbytes, addr)) => match RawMessage::unpack(&buf[0..nbytes]) {
                    Ok(raw) => {
                        if raw.frame_addr.target == 0 {
                            continue;
                        }
                        if let Ok(mut bulbs) = receiver_bulbs.lock() {
                            let bulb = bulbs
                                .entry(raw.frame_addr.target)
                                .and_modify(|bulb| bulb.update(addr))
                                .or_insert_with(|| {
                                    BulbInfo::new(source, raw.frame_addr.target, addr)
                                });
                            if let Err(e) = Self::handle_message(raw, bulb) {
                                log::error!("Error handling message from {}: {}", addr, e)
                            }
                        }
                    }
                    Err(e) => log::error!("Error unpacking raw message from {}: {}", addr, e),
                },
                Err(e) => panic!("recv_from err {:?}", e),
            }
        }
    }

    pub fn discover(&mut self) -> Result<(), failure::Error> {
        log::debug!("Doing discovery");

        let opts = BuildOptions {
            source: self.source,
            ..Default::default()
        };
        let rawmsg = RawMessage::build(&opts, Message::GetService)?;
        let bytes = rawmsg.pack()?;

        for addr in get_if_addrs()? {
            if let IfAddr::V4(Ifv4Addr {
                broadcast: Some(bcast),
                ..
            }) = addr.addr
            {
                if addr.ip().is_loopback() {
                    continue;
                }
                let addr = SocketAddr::new(IpAddr::V4(bcast), 56700);
                log::debug!("Discovering bulbs on LAN {:?}", addr);
                self.sock.send_to(&bytes, addr)?;
            }
        }

        self.last_discovery = Instant::now();

        Ok(())
    }

    pub fn refresh(&self) {
        if let Ok(mut bulbs) = self.bulbs.lock() {
            let bulbs = bulbs.values_mut();
            for bulb in bulbs {
                if let Err(e) = bulb.query_for_missing_info(&self.sock) {
                    log::error!("Error querying bulb: {}", e);
                }
            }
        }
    }

    fn send_message(&self, bulb: &&BulbInfo, message: Message) -> Result<usize, std::io::Error> {
        let target = bulb.addr;
        let opts = BuildOptions {
            target: Some(bulb.target),
            source: bulb.source,
            ack_required: true,
            res_required: true,
            sequence: 0,
        };
        let raw = RawMessage::build(&opts, message).expect("Failed to build message");
        let bytes = raw.pack().expect("Failed to pack message");
        self.sock.send_to(&bytes, target)
    }

    pub fn set_power(&self, bulb: &&BulbInfo, level: u16) -> Result<usize, std::io::Error> {
        self.send_message(bulb, Message::LightSetPower { level, duration: 0 })
    }

    pub fn set_group_power(
        &self,
        group: &GroupInfo,
        bulbs: &MutexGuard<HashMap<u64, BulbInfo>>,
        level: u16,
    ) -> Result<usize, std::io::Error> {
        let bulbs: Vec<&BulbInfo> = group.get_bulbs(bulbs);
        bulbs.into_iter().map(|b| self.set_power(&b, level)).sum()
    }

    pub fn set_color(&self, bulb: &&BulbInfo, color: HSBK) -> Result<usize, std::io::Error> {
        self.send_message(
            bulb,
            Message::LightSetColor {
                reserved: 0,
                color,
                duration: 0,
            },
        )
    }

    pub fn get_groups(&self) -> Vec<GroupInfo> {
        let mut groups = Vec::new();
        if let Ok(bulbs) = self.bulbs.lock() {
            for bulb in bulbs.values() {
                if let Some(group) = &bulb.group.data {
                    groups.push(group.clone());
                }
            }
        }
        groups
    }

    pub fn set_group_color(
        &self,
        group: &GroupInfo,
        color: HSBK,
        bulbs: &MutexGuard<HashMap<u64, BulbInfo>>,
    ) -> Result<usize, std::io::Error> {
        let bulbs = group.get_bulbs(bulbs);
        for bulb in bulbs {
            self.set_color(&bulb, color)?;
        }
        Ok(0)
    }

    pub fn avg_group_color(
        &self,
        group: &GroupInfo,
        bulbs: &MutexGuard<HashMap<u64, BulbInfo>>,
    ) -> HSBK {
        let bulbs = group.get_bulbs(bulbs);
        let mut colors = Vec::new();
        for bulb in bulbs {
            if let Some(color) = bulb.get_color() {
                colors.push(color);
            }
        }
        if colors.is_empty() {
            return default_hsbk();
        }
        // Use u32 to avoid overflow
        let avg = colors.iter().fold(HSBK32::default(), |acc, c| HSBK32 {
            hue: acc.hue.saturating_add(c.hue as u32),
            saturation: acc.saturation.saturating_add(c.saturation as u32),
            brightness: acc.brightness.saturating_add(c.brightness as u32),
            kelvin: acc.kelvin.saturating_add(c.kelvin as u32),
        });
        let avg = HSBK32 {
            hue: avg.hue / colors.len() as u32,
            saturation: avg.saturation / colors.len() as u32,
            brightness: avg.brightness / colors.len() as u32,
            kelvin: avg.kelvin / colors.len() as u32,
        };
        avg.into()
    }
}
