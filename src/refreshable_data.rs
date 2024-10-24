use lifx_core::Message;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct RefreshableData<T> {
    pub data: Option<T>,
    pub max_age: Duration,
    pub last_updated: Instant,
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
