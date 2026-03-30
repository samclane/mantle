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

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_msg() -> Message {
        Message::GetService
    }

    #[test]
    fn empty_has_no_data() {
        let rd: RefreshableData<u32> = RefreshableData::empty(Duration::from_secs(60), dummy_msg());
        assert!(rd.data.is_none());
    }

    #[test]
    fn empty_needs_refresh_immediately() {
        let rd: RefreshableData<u32> = RefreshableData::empty(Duration::from_secs(60), dummy_msg());
        assert!(rd.needs_refresh());
    }

    #[test]
    fn new_has_data() {
        let rd = RefreshableData::new(42u32, Duration::from_secs(60), dummy_msg());
        assert_eq!(rd.data, Some(42));
    }

    #[test]
    fn new_does_not_need_refresh_immediately() {
        let rd = RefreshableData::new(42u32, Duration::from_secs(60), dummy_msg());
        assert!(!rd.needs_refresh());
    }

    #[test]
    fn needs_refresh_with_zero_max_age() {
        let rd = RefreshableData::new(42u32, Duration::ZERO, dummy_msg());
        std::thread::sleep(Duration::from_millis(1));
        assert!(rd.needs_refresh());
    }

    #[test]
    fn update_replaces_data() {
        let mut rd: RefreshableData<u32> =
            RefreshableData::empty(Duration::from_secs(60), dummy_msg());
        assert!(rd.data.is_none());
        rd.update(99);
        assert_eq!(rd.data, Some(99));
    }

    #[test]
    fn update_resets_timer() {
        let mut rd = RefreshableData::new(1u32, Duration::ZERO, dummy_msg());
        std::thread::sleep(Duration::from_millis(5));
        assert!(rd.needs_refresh());
        rd.update(2);
        rd.max_age = Duration::from_secs(60);
        assert!(!rd.needs_refresh());
    }

    #[test]
    fn as_ref_returns_some_when_populated() {
        let rd = RefreshableData::new(42u32, Duration::from_secs(60), dummy_msg());
        assert_eq!(rd.as_ref(), Some(&42));
    }

    #[test]
    fn as_ref_returns_none_when_empty() {
        let rd: RefreshableData<u32> = RefreshableData::empty(Duration::from_secs(60), dummy_msg());
        assert_eq!(rd.as_ref(), None);
    }

    #[test]
    fn stores_refresh_msg() {
        let rd: RefreshableData<u32> =
            RefreshableData::empty(Duration::from_secs(60), Message::GetLabel);
        match rd.refresh_msg {
            Message::GetLabel => {}
            _ => panic!("Expected GetLabel"),
        }
    }
}
