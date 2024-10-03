use super::input_item::InputItem;
use super::input_listener::BackgroundCallback;
use rdev::{Button, Event, Key};
use std::time::Instant;
use std::{collections::BTreeSet, sync::Mutex};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MousePosition {
    pub x: i32,
    pub y: i32,
}

pub struct SharedInputState {
    pub last_mouse_position: Mutex<Option<MousePosition>>,
    pub last_click_time: Mutex<Option<Instant>>,
    pub keys_pressed: Mutex<BTreeSet<InputItem>>,
    pub callbacks: Mutex<Vec<BackgroundCallback>>,
}

impl SharedInputState {
    pub fn new() -> Self {
        SharedInputState {
            last_mouse_position: Mutex::new(None),
            last_click_time: Mutex::new(None),
            keys_pressed: Mutex::new(BTreeSet::new()),
            callbacks: Mutex::new(Vec::new()),
        }
    }

    pub fn update_input_key_press(&self, input_key: InputItem) {
        let mut keys = self
            .keys_pressed
            .lock()
            .expect("Failed to lock keys_pressed mutex");
        keys.insert(input_key);
    }

    pub fn update_input_key_release(&self, input_key: InputItem) {
        let mut keys = self
            .keys_pressed
            .lock()
            .expect("Failed to lock keys_pressed mutex");
        keys.remove(&input_key);
    }

    pub fn update_mouse_position(&self, x: i32, y: i32) {
        let mut pos = self
            .last_mouse_position
            .lock()
            .expect("Failed to lock last_mouse_position mutex");
        *pos = Some(MousePosition { x, y });
    }

    pub fn update_button_press(&self, button: Button) {
        self.update_input_key_press(InputItem::Button(button));

        let mut time = self
            .last_click_time
            .lock()
            .expect("Failed to lock last_click_time mutex");
        *time = Some(Instant::now());
    }

    pub fn update_button_release(&self, button: Button) {
        self.update_input_key_release(InputItem::Button(button));
    }

    pub fn update_key_press(&self, key: Key) {
        self.update_input_key_press(InputItem::Key(key));
    }

    pub fn update_key_release(&self, key: Key) {
        self.update_input_key_release(InputItem::Key(key));
    }

    pub fn execute_callbacks(&self, event: &Event) {
        let callbacks = self
            .callbacks
            .lock()
            .expect("Failed to lock callbacks mutex");
        for callback in callbacks.iter() {
            callback(event.clone());
        }
    }
}

impl Default for SharedInputState {
    fn default() -> Self {
        Self::new()
    }
}
