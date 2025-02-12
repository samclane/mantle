use crate::serializers::{deserialize_instant, serialize_instant, MessageDef};
use lifx_core::Message;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// A wrapper around a piece of data that can be refreshed after a certain
/// amount of time has passed. This is useful for caching data that is
/// expensive to fetch or that doesn't need to be fetched often.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshableData<T> {
    pub data: Option<T>,
    pub max_age: Duration,
    #[serde(
        serialize_with = "serialize_instant",
        deserialize_with = "deserialize_instant"
    )]
    pub last_updated: Instant,
    #[serde(with = "MessageDef")]
    pub refresh_msg: Message,
}

impl<T> RefreshableData<T> {
    pub fn empty(max_age: Duration, refresh_msg: Message) -> RefreshableData<T> {
        RefreshableData {
            data: None,
            max_age,
            last_updated: Instant::now(),
            refresh_msg,
        }
    }

    pub fn update(&mut self, data: T) {
        self.data = Some(data);
        self.last_updated = Instant::now()
    }

    pub fn needs_refresh(&self) -> bool {
        self.data.is_none() || self.last_updated.elapsed() > self.max_age
    }

    pub fn as_ref(&self) -> Option<&T> {
        self.data.as_ref()
    }

    pub fn new(data: T, max_age: Duration, refresh_msg: Message) -> RefreshableData<T> {
        RefreshableData {
            data: Some(data),
            max_age,
            last_updated: Instant::now(),
            refresh_msg,
        }
    }
}
