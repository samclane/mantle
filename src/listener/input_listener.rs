use super::input_action::InputAction;
use super::input_item::InputItem;
use super::shared_input_state::{MousePosition, SharedInputState};
use log::error;
use rdev::{listen, Button, Event, EventType, Key};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::thread::{spawn, JoinHandle};
use std::time::Instant;

pub type BackgroundCallback = Box<dyn Fn(Event) + Send>;

#[derive(Clone)]
pub struct InputListener {
    pub state: Arc<SharedInputState>,
}

impl InputListener {
    pub fn new() -> Self {
        InputListener {
            state: Arc::new(SharedInputState::new()),
        }
    }

    pub fn get_last_mouse_position(&self) -> Option<MousePosition> {
        let pos = self
            .state
            .last_mouse_position
            .lock()
            .expect("Failed to lock last_mouse_position mutex");
        *pos
    }

    pub fn get_last_click_time(&self) -> Option<Instant> {
        let time = self
            .state
            .last_click_time
            .lock()
            .expect("Failed to lock last_click_time mutex");
        *time
    }

    pub fn is_input_key_pressed(&self, input_key: InputItem) -> bool {
        let keys = self
            .state
            .keys_pressed
            .lock()
            .expect("Failed to lock keys_pressed mutex");
        keys.contains(&input_key)
    }

    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.is_input_key_pressed(InputItem::Key(key))
    }

    pub fn is_button_pressed(&self, button: Button) -> bool {
        self.is_input_key_pressed(InputItem::Button(button))
    }

    pub fn get_keys_pressed(&self) -> BTreeSet<InputItem> {
        let keys = self
            .state
            .keys_pressed
            .lock()
            .expect("Failed to lock keys_pressed mutex");
        keys.clone()
    }

    pub fn is_input_action_pressed(&self, input_action: &InputAction) -> bool {
        let keys = self
            .state
            .keys_pressed
            .lock()
            .expect("Failed to lock keys_pressed mutex");
        input_action.is_subset(&keys)
    }

    pub fn add_callback(&self, callback: BackgroundCallback) {
        let mut callbacks = self
            .state
            .callbacks
            .lock()
            .expect("Failed to lock callbacks mutex");
        callbacks.push(callback);
    }

    pub fn start(&self) -> JoinHandle<()> {
        let state = Arc::clone(&self.state);

        spawn(move || {
            if let Err(e) = listen(move |event| {
                match event.event_type {
                    EventType::MouseMove { x, y } => {
                        state.update_mouse_position(x as i32, y as i32);
                    }
                    EventType::ButtonPress(button) => {
                        state.update_button_press(button);
                    }
                    EventType::ButtonRelease(button) => {
                        state.update_button_release(button);
                    }
                    EventType::KeyPress(key) => {
                        state.update_key_press(key);
                    }
                    EventType::KeyRelease(key) => {
                        state.update_key_release(key);
                    }
                    _ => {}
                }

                state.execute_callbacks(&event);
            }) {
                error!("Error in listen: {:?}", e);
            }
        })
    }
}

impl Default for InputListener {
    fn default() -> Self {
        Self::new()
    }
}
