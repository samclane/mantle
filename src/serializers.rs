use lifx_core::HSBK;
use lifx_core::{
    ApplicationRequest, EchoPayload, LastHevCycleResult, LifxIdent, LifxString, Message,
    MultiZoneEffectType, PowerLevel, Service, Waveform,
};
use serde::{Deserialize, Serialize};
use std::ffi::CString;

#[derive(Serialize, Deserialize)]
#[serde(remote = "Service")]
pub enum ServiceDef {
    UDP = 1,
    Reserved1 = 2,
    Reserved2 = 3,
    Reserved3 = 4,
    Reserved4 = 5,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "PowerLevel")]
pub enum PowerLevelDef {
    Standby = 0,
    Enabled = 65535,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "LifxIdent")]
pub struct LifxIdentDef([u8; 16]);

pub fn serialize_lifx_string<S>(data: &LifxString, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    data.cstr().serialize(serializer)
}

pub fn deserialize_lifx_string<'de, D>(deserializer: D) -> Result<LifxString, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let cstr = CString::deserialize(deserializer)?;
    Ok(LifxString::new(&cstr))
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "EchoPayload")]
pub struct EchoPayloadDef(
    #[serde(
        serialize_with = "serialize_bytes",
        deserialize_with = "deserialize_bytes"
    )]
    [u8; 64],
);

fn serialize_bytes<S>(bytes: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_bytes(bytes)
}

fn deserialize_bytes<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let bytes: Vec<u8> = serde::Deserialize::deserialize(deserializer)?;
    let mut array = [0u8; 64];
    array.copy_from_slice(&bytes[..64]);
    Ok(array)
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "HSBK")]
pub struct HSBKDef {
    pub hue: u16,
    pub saturation: u16,
    pub brightness: u16,
    pub kelvin: u16,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Waveform")]
pub enum WaveformDef {
    Saw = 0,
    Sine = 1,
    HalfSign = 2,
    Triangle = 3,
    Pulse = 4,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "LastHevCycleResult")]
pub enum LastHevCycleResultDef {
    Success = 0,
    Busy = 1,
    InterruptedByReset = 2,
    InterruptedByHomekit = 3,
    InterruptedByLan = 4,
    InterruptedByCloud = 5,
    None = 255,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "ApplicationRequest")]
pub enum ApplicationRequestDef {
    NoApply = 0,
    Apply = 1,
    ApplyOnly = 2,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "MultiZoneEffectType")]
pub enum MultiZoneEffectTypeDef {
    Off = 0,
    Move = 1,
    Reserved1 = 2,
    Reserved2 = 3,
}

type ColorBox = Box<[HSBK; 82]>;

fn serialize_color_box<S>(colors: &ColorBox, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    // make colors a Vec of u16s so we can serialize it
    let colors: Vec<u16> = colors
        .iter()
        .flat_map(|color| vec![color.hue, color.saturation, color.brightness, color.kelvin])
        .collect();
    colors.serialize(serializer)
}

fn deserialize_color_box<'de, D>(deserializer: D) -> Result<ColorBox, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let colors: Vec<u16> = serde::Deserialize::deserialize(deserializer)?;
    let mut color_iter = colors.iter();
    let mut color_box = Vec::with_capacity(82);
    for _ in 0..82 {
        color_box.push(HSBK {
            hue: *color_iter.next().unwrap(),
            saturation: *color_iter.next().unwrap(),
            brightness: *color_iter.next().unwrap(),
            kelvin: *color_iter.next().unwrap(),
        });
    }
    let color_array: [HSBK; 82] = color_box
        .try_into()
        .map_err(|_| serde::de::Error::custom("Incorrect length"))?;
    Ok(Box::new(color_array))
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Message")]
pub enum MessageDef {
    GetService,

    StateService {
        #[serde(with = "ServiceDef")]
        service: Service,
        port: u32,
    },

    GetHostInfo,

    StateHostInfo {
        signal: f32,
        tx: u32,
        rx: u32,
        reserved: i16,
    },

    GetHostFirmware,

    StateHostFirmware {
        build: u64,
        reserved: u64,
        version_minor: u16,
        version_major: u16,
    },

    GetWifiInfo,

    StateWifiInfo {
        signal: f32,
        reserved6: u32,
        reserved7: u32,
        reserved: i16,
    },

    GetWifiFirmware,

    StateWifiFirmware {
        build: u64,
        reserved: u64,
        version_minor: u16,
        version_major: u16,
    },

    GetPower,

    SetPower {
        #[serde(with = "PowerLevelDef")]
        level: PowerLevel,
    },

    StatePower {
        level: u16,
    },

    GetLabel,

    SetLabel {
        #[serde(
            serialize_with = "serialize_lifx_string",
            deserialize_with = "deserialize_lifx_string"
        )]
        label: LifxString,
    },

    StateLabel {
        #[serde(
            serialize_with = "serialize_lifx_string",
            deserialize_with = "deserialize_lifx_string"
        )]
        label: LifxString,
    },

    GetVersion,

    StateVersion {
        vendor: u32,
        product: u32,
        reserved: u32,
    },

    GetInfo,

    StateInfo {
        time: u64,
        uptime: u64,
        downtime: u64,
    },

    Acknowledgement {
        seq: u8,
    },

    GetLocation,

    SetLocation {
        #[serde(with = "LifxIdentDef")]
        location: LifxIdent,
        #[serde(
            serialize_with = "serialize_lifx_string",
            deserialize_with = "deserialize_lifx_string"
        )]
        label: LifxString,
        updated_at: u64,
    },

    StateLocation {
        #[serde(with = "LifxIdentDef")]
        location: LifxIdent,
        #[serde(
            serialize_with = "serialize_lifx_string",
            deserialize_with = "deserialize_lifx_string"
        )]
        label: LifxString,
        updated_at: u64,
    },

    GetGroup,

    SetGroup {
        #[serde(with = "LifxIdentDef")]
        group: LifxIdent,
        #[serde(
            serialize_with = "serialize_lifx_string",
            deserialize_with = "deserialize_lifx_string"
        )]
        label: LifxString,
        updated_at: u64,
    },

    StateGroup {
        #[serde(with = "LifxIdentDef")]
        group: LifxIdent,
        #[serde(
            serialize_with = "serialize_lifx_string",
            deserialize_with = "deserialize_lifx_string"
        )]
        label: LifxString,
        updated_at: u64,
    },

    EchoRequest {
        #[serde(with = "EchoPayloadDef")]
        payload: EchoPayload,
    },

    EchoResponse {
        #[serde(with = "EchoPayloadDef")]
        payload: EchoPayload,
    },

    LightGet,

    LightSetColor {
        reserved: u8,
        #[serde(with = "HSBKDef")]
        color: HSBK,
        duration: u32,
    },

    SetWaveform {
        reserved: u8,
        transient: bool,
        #[serde(with = "HSBKDef")]
        color: HSBK,
        period: u32,
        cycles: f32,
        skew_ratio: i16,
        #[serde(with = "WaveformDef")]
        waveform: Waveform,
    },

    LightState {
        #[serde(with = "HSBKDef")]
        color: HSBK,
        reserved: i16,
        power: u16,
        #[serde(
            serialize_with = "serialize_lifx_string",
            deserialize_with = "deserialize_lifx_string"
        )]
        label: LifxString,
        reserved2: u64,
    },

    LightGetPower,

    LightSetPower {
        level: u16,
        duration: u32,
    },

    LightStatePower {
        level: u16,
    },

    SetWaveformOptional {
        reserved: u8,
        transient: bool,
        #[serde(with = "HSBKDef")]
        color: HSBK,
        period: u32,
        cycles: f32,
        skew_ratio: i16,
        #[serde(with = "WaveformDef")]
        waveform: Waveform,
        set_hue: bool,
        set_saturation: bool,
        set_brightness: bool,
        set_kelvin: bool,
    },

    LightGetInfrared,

    LightStateInfrared {
        brightness: u16,
    },

    LightSetInfrared {
        brightness: u16,
    },

    LightGetHevCycle,

    LightSetHevCycle {
        enable: bool,
        duration: u32,
    },

    LightStateHevCycle {
        duration: u32,
        remaining: u32,
        last_power: bool,
    },

    LightGetHevCycleConfiguration,

    LightSetHevCycleConfiguration {
        indication: bool,
        duration: u32,
    },

    LightStateHevCycleConfiguration {
        indication: bool,
        duration: u32,
    },

    LightGetLastHevCycleResult,

    LightStateLastHevCycleResult {
        #[serde(with = "LastHevCycleResultDef")]
        result: LastHevCycleResult,
    },

    SetColorZones {
        start_index: u8,
        end_index: u8,
        #[serde(with = "HSBKDef")]
        color: HSBK,
        duration: u32,
        #[serde(with = "ApplicationRequestDef")]
        apply: ApplicationRequest,
    },

    GetColorZones {
        start_index: u8,
        end_index: u8,
    },

    StateZone {
        count: u8,
        index: u8,
        #[serde(with = "HSBKDef")]
        color: HSBK,
    },

    StateMultiZone {
        count: u8,
        index: u8,
        #[serde(with = "HSBKDef")]
        color0: HSBK,
        #[serde(with = "HSBKDef")]
        color1: HSBK,
        #[serde(with = "HSBKDef")]
        color2: HSBK,
        #[serde(with = "HSBKDef")]
        color3: HSBK,
        #[serde(with = "HSBKDef")]
        color4: HSBK,
        #[serde(with = "HSBKDef")]
        color5: HSBK,
        #[serde(with = "HSBKDef")]
        color6: HSBK,
        #[serde(with = "HSBKDef")]
        color7: HSBK,
    },

    GetMultiZoneEffect,

    SetMultiZoneEffect {
        instance_id: u32,
        #[serde(with = "MultiZoneEffectTypeDef")]
        typ: MultiZoneEffectType,
        reserved: u16,
        speed: u32,
        duration: u64,
        reserved7: u32,
        reserved8: u32,
        parameters: [u32; 8],
    },

    StateMultiZoneEffect {
        instance_id: u32,
        #[serde(with = "MultiZoneEffectTypeDef")]
        typ: MultiZoneEffectType,
        reserved: u16,
        speed: u32,
        duration: u64,
        reserved7: u32,
        reserved8: u32,
        parameters: [u32; 8],
    },

    SetExtendedColorZones {
        duration: u32,
        #[serde(with = "ApplicationRequestDef")]
        apply: ApplicationRequest,
        zone_index: u16,
        colors_count: u8,
        #[serde(
            serialize_with = "serialize_color_box",
            deserialize_with = "deserialize_color_box"
        )]
        colors: Box<[HSBK; 82]>,
    },

    GetExtendedColorZone,

    StateExtendedColorZones {
        zones_count: u16,
        zone_index: u16,
        colors_count: u8,
        #[serde(
            serialize_with = "serialize_color_box",
            deserialize_with = "deserialize_color_box"
        )]
        colors: Box<[HSBK; 82]>,
    },

    RelayGetPower {
        relay_index: u8,
    },

    RelaySetPower {
        relay_index: u8,
        level: u16,
    },

    RelayStatePower {
        relay_index: u8,
        level: u16,
    },
}
