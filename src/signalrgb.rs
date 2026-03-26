use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 16038;
const API_PREFIX: &str = "/api/v1";
const POLL_INTERVAL: Duration = Duration::from_secs(3);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(800);
const READ_TIMEOUT: Duration = Duration::from_secs(3);

// ---------------------------------------------------------------------------
// API response types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ApiResponse<T> {
    pub status: String,
    pub data: Option<T>,
    pub errors: Option<Vec<ApiError>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub title: String,
    pub detail: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CurrentEffectData {
    pub id: String,
    pub attributes: CurrentEffectAttributes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CurrentEffectAttributes {
    pub name: String,
    pub enabled: Option<bool>,
    pub global_brightness: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EffectListData {
    pub items: Vec<EffectItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EffectItem {
    pub id: String,
    pub attributes: EffectItemAttributes,
    pub links: Option<EffectLinks>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EffectItemAttributes {
    pub name: String,
    pub description: Option<String>,
    pub image: Option<String>,
    pub publisher: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EffectLinks {
    #[serde(rename = "self")]
    pub self_link: Option<String>,
    pub apply: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PresetListData {
    pub items: Vec<PresetItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PresetItem {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CanvasStateData {
    pub attributes: CanvasAttributes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CanvasAttributes {
    pub enabled: Option<bool>,
    pub global_brightness: Option<u32>,
}

// ---------------------------------------------------------------------------
// Cached snapshot of SignalRGB state (shared across threads)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct SignalRGBState {
    pub connected: bool,
    pub current_effect_name: Option<String>,
    pub current_effect_id: Option<String>,
    pub enabled: Option<bool>,
    pub global_brightness: Option<u32>,
    pub effects: Vec<EffectItem>,
    pub presets: Vec<PresetItem>,
    pub last_refresh: Option<Instant>,
    pub last_error: Option<String>,
}

// ---------------------------------------------------------------------------
// Configuration (persisted via serde)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignalRGBConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
}

impl Default for SignalRGBConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
        }
    }
}

// ---------------------------------------------------------------------------
// Manager — all network I/O runs on background threads
// ---------------------------------------------------------------------------

pub struct SignalRGBManager {
    pub state: Arc<Mutex<SignalRGBState>>,
    pub config: SignalRGBConfig,
    poll_in_flight: Arc<Mutex<bool>>,
    effects_fetched: Arc<Mutex<bool>>,
}

impl Clone for SignalRGBManager {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            config: self.config.clone(),
            poll_in_flight: self.poll_in_flight.clone(),
            effects_fetched: self.effects_fetched.clone(),
        }
    }
}

impl Default for SignalRGBManager {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(SignalRGBState::default())),
            config: SignalRGBConfig::default(),
            poll_in_flight: Arc::new(Mutex::new(false)),
            effects_fetched: Arc::new(Mutex::new(false)),
        }
    }
}

impl SignalRGBManager {
    pub fn new(config: SignalRGBConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    fn base_url(config: &SignalRGBConfig) -> String {
        format!("http://{}:{}{}", config.host, config.port, API_PREFIX)
    }

    // -- non-blocking poll (spawns a thread if none in flight) ---------------

    pub fn kick_refresh(&self) {
        if !self.config.enabled {
            return;
        }

        if let Ok(mut in_flight) = self.poll_in_flight.lock() {
            if *in_flight {
                return;
            }
            *in_flight = true;
        } else {
            return;
        }

        let state = self.state.clone();
        let in_flight = self.poll_in_flight.clone();
        let effects_fetched = self.effects_fetched.clone();
        let config = self.config.clone();

        thread::spawn(move || {
            let base = Self::base_url(&config);

            match Self::fetch_current(&base) {
                Ok(data) => {
                    if let Ok(mut s) = state.lock() {
                        s.connected = true;
                        s.current_effect_name = Some(data.attributes.name);
                        s.current_effect_id = Some(data.id);
                        s.enabled = data.attributes.enabled;
                        s.global_brightness = data.attributes.global_brightness;
                        s.last_refresh = Some(Instant::now());
                        s.last_error = None;
                    }
                }
                Err(e) => {
                    if let Ok(mut s) = state.lock() {
                        s.connected = false;
                        s.last_error = Some(e);
                        s.last_refresh = Some(Instant::now());
                    }
                }
            }

            let should_fetch = effects_fetched.lock().map(|f| !*f).unwrap_or(false);

            if should_fetch {
                if let Ok(items) = Self::fetch_effects(&base) {
                    if let Ok(mut s) = state.lock() {
                        s.effects = items;
                    }
                    if let Ok(mut f) = effects_fetched.lock() {
                        *f = true;
                    }
                }
            }

            if let Ok(mut f) = in_flight.lock() {
                *f = false;
            }
        });
    }

    pub fn needs_refresh(&self) -> bool {
        if !self.config.enabled {
            return false;
        }
        if let Ok(s) = self.state.lock() {
            s.last_refresh
                .map(|t| t.elapsed() > POLL_INTERVAL)
                .unwrap_or(true)
        } else {
            true
        }
    }

    // -- fire-and-forget commands (each spawns its own thread) ---------------

    pub fn set_brightness(&self, brightness: u32) {
        if let Ok(mut s) = self.state.lock() {
            s.global_brightness = Some(brightness);
        }
        let base = Self::base_url(&self.config);
        thread::spawn(move || {
            let url = format!("{}/lighting/global_brightness", base);
            let body = serde_json::json!({ "global_brightness": brightness });
            if let Err(e) = ureq::patch(&url).timeout(CONNECT_TIMEOUT).send_json(&body) {
                log::error!("SignalRGB set_brightness: {}", e);
            }
        });
    }

    pub fn set_enabled(&self, enabled: bool) {
        if let Ok(mut s) = self.state.lock() {
            s.enabled = Some(enabled);
        }
        let base = Self::base_url(&self.config);
        thread::spawn(move || {
            let url = format!("{}/lighting/enabled", base);
            let body = serde_json::json!({ "enabled": enabled });
            if let Err(e) = ureq::patch(&url).timeout(CONNECT_TIMEOUT).send_json(&body) {
                log::error!("SignalRGB set_enabled: {}", e);
            }
        });
    }

    pub fn apply_effect(&self, effect_id: &str) {
        let url = format!(
            "{}/lighting/effects/{}/apply",
            Self::base_url(&self.config),
            urlencoded(effect_id)
        );
        thread::spawn(move || {
            if let Err(e) = ureq::post(&url).timeout(CONNECT_TIMEOUT).call() {
                log::error!("SignalRGB apply_effect: {}", e);
            }
        });
    }

    pub fn next_effect(&self) {
        let url = format!("{}/lighting/next", Self::base_url(&self.config));
        thread::spawn(move || {
            if let Err(e) = ureq::post(&url).timeout(CONNECT_TIMEOUT).call() {
                log::error!("SignalRGB next_effect: {}", e);
            }
        });
    }

    pub fn previous_effect(&self) {
        let url = format!("{}/lighting/previous", Self::base_url(&self.config));
        thread::spawn(move || {
            if let Err(e) = ureq::post(&url).timeout(CONNECT_TIMEOUT).call() {
                log::error!("SignalRGB previous_effect: {}", e);
            }
        });
    }

    pub fn shuffle_effect(&self) {
        let url = format!("{}/lighting/shuffle", Self::base_url(&self.config));
        thread::spawn(move || {
            if let Err(e) = ureq::post(&url).timeout(CONNECT_TIMEOUT).call() {
                log::error!("SignalRGB shuffle_effect: {}", e);
            }
        });
    }

    pub fn apply_preset(&self, effect_id: &str, preset_name: &str) {
        let url = format!(
            "{}/lighting/effects/{}/presets",
            Self::base_url(&self.config),
            urlencoded(effect_id)
        );
        let body = serde_json::json!({ "preset": preset_name });
        thread::spawn(move || {
            if let Err(e) = ureq::patch(&url).timeout(CONNECT_TIMEOUT).send_json(&body) {
                log::error!("SignalRGB apply_preset: {}", e);
            }
        });
    }

    pub fn reload_effects(&mut self) {
        if let Ok(mut f) = self.effects_fetched.lock() {
            *f = false;
        }
    }

    // -- internal helpers (run on background threads) ------------------------

    fn fetch_current(base: &str) -> Result<CurrentEffectData, String> {
        let url = format!("{}/lighting", base);
        let resp: ApiResponse<CurrentEffectData> = ureq::get(&url)
            .timeout(CONNECT_TIMEOUT)
            .call()
            .map_err(|e| e.to_string())?
            .into_json()
            .map_err(|e| e.to_string())?;
        resp.data.ok_or_else(|| "No data in response".to_string())
    }

    fn fetch_effects(base: &str) -> Result<Vec<EffectItem>, String> {
        let url = format!("{}/lighting/effects", base);
        let resp: ApiResponse<EffectListData> = ureq::get(&url)
            .timeout(READ_TIMEOUT)
            .call()
            .map_err(|e| e.to_string())?
            .into_json()
            .map_err(|e| e.to_string())?;
        Ok(resp.data.map(|d| d.items).unwrap_or_default())
    }
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
        .replace('#', "%23")
        .replace('&', "%26")
        .replace('?', "%3F")
}
