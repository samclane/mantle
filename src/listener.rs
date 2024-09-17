use log::error;
use rdev::{listen, Event, EventType};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

type Callback = Box<dyn Fn(Event) + Send>;

#[derive(Clone, Copy)]
pub struct MousePosition {
    pub x: i32,
    pub y: i32,
}

pub struct SharedInputState {
    last_mouse_position: Mutex<Option<MousePosition>>,
    last_click_time: Mutex<Option<Instant>>,
    button_pressed: Mutex<Option<rdev::Button>>,
    last_button_pressed: Mutex<Option<rdev::Button>>,
    keys_pressed: Mutex<Vec<rdev::Key>>,
    last_keys_pressed: Mutex<Vec<rdev::Key>>,
    callbacks: Mutex<Vec<Callback>>,
}

impl SharedInputState {
    fn new() -> Self {
        SharedInputState {
            last_mouse_position: Mutex::new(None),
            last_click_time: Mutex::new(None),
            button_pressed: Mutex::new(None),
            last_button_pressed: Mutex::new(None),
            keys_pressed: Mutex::new(Vec::new()),
            last_keys_pressed: Mutex::new(Vec::new()),
            callbacks: Mutex::new(Vec::new()),
        }
    }

    fn update_mouse_position(&self, x: i32, y: i32) {
        match self.last_mouse_position.lock() {
            Ok(mut pos) => {
                *pos = Some(MousePosition { x, y });
            }
            Err(e) => {
                error!("Failed to lock last_mouse_position mutex: {}", e);
            }
        }
    }

    fn update_button_press(&self, button: rdev::Button) {
        if let Err(e) = self.button_pressed.lock().map(|mut pressed| {
            *pressed = Some(button);
        }) {
            error!("Failed to lock button_pressed mutex: {}", e);
        }

        if let Err(e) = self.last_click_time.lock().map(|mut time| {
            *time = Some(Instant::now());
        }) {
            error!("Failed to lock last_click_time mutex: {}", e);
        }

        if let Err(e) = self.last_button_pressed.lock().map(|mut last| {
            *last = Some(button);
        }) {
            error!("Failed to lock last_button_pressed mutex: {}", e);
        }
    }

    fn update_key_press(&self, key: rdev::Key) {
        match self.keys_pressed.lock() {
            Ok(mut keys) => {
                keys.push(key);

                if let Ok(mut last) = self.last_keys_pressed.lock() {
                    *last = keys.clone();
                } else {
                    error!("Failed to lock last_keys_pressed mutex");
                }
            }
            Err(e) => {
                error!("Failed to lock keys_pressed mutex: {}", e);
            }
        }
    }

    fn update_key_release(&self, key: rdev::Key) {
        match self.keys_pressed.lock() {
            Ok(mut keys) => {
                keys.retain(|&k| k != key);
            }
            Err(e) => {
                error!("Failed to lock keys_pressed mutex: {}", e);
            }
        }
    }

    fn update_button_release(&self) {
        if let Err(e) = self.button_pressed.lock().map(|mut pressed| {
            *pressed = None;
        }) {
            error!("Failed to lock button_pressed mutex: {}", e);
        }
    }

    fn execute_callbacks(&self, event: &Event) {
        match self.callbacks.lock() {
            Ok(callbacks) => {
                for callback in callbacks.iter() {
                    callback(event.clone());
                }
            }
            Err(e) => {
                error!("Failed to lock callbacks mutex: {}", e);
            }
        }
    }

    fn add_callback(&self, callback: Callback) {
        match self.callbacks.lock() {
            Ok(mut callbacks) => {
                callbacks.push(callback);
            }
            Err(e) => {
                error!("Failed to lock callbacks mutex: {}", e);
            }
        }
    }
}

pub struct InputListener {
    state: Arc<SharedInputState>,
}

impl InputListener {
    pub fn new() -> Self {
        InputListener {
            state: Arc::new(SharedInputState::new()),
        }
    }

    pub fn get_last_mouse_position(&self) -> Option<MousePosition> {
        match self.state.last_mouse_position.lock() {
            Ok(guard) => *guard,
            Err(e) => {
                error!("Failed to lock last_mouse_position mutex: {}", e);
                None
            }
        }
    }

    pub fn get_last_click_time(&self) -> Option<Instant> {
        match self.state.last_click_time.lock() {
            Ok(guard) => *guard,
            Err(e) => {
                error!("Failed to lock last_click_time mutex: {}", e);
                None
            }
        }
    }

    pub fn get_last_button_pressed(&self) -> Option<rdev::Button> {
        match self.state.last_button_pressed.lock() {
            Ok(guard) => *guard,
            Err(e) => {
                error!("Failed to lock last_button_pressed mutex: {}", e);
                None
            }
        }
    }

    pub fn is_button_pressed(&self, button: rdev::Button) -> bool {
        match self.state.button_pressed.lock() {
            Ok(guard) => guard.map_or(false, |b| b == button),
            Err(e) => {
                error!("Failed to lock button_pressed mutex: {}", e);
                false
            }
        }
    }

    pub fn is_key_pressed(&self, key: rdev::Key) -> bool {
        match self.state.keys_pressed.lock() {
            Ok(guard) => guard.contains(&key),
            Err(e) => {
                error!("Failed to lock keys_pressed mutex: {}", e);
                false
            }
        }
    }

    pub fn get_keys_pressed(&self) -> Vec<rdev::Key> {
        match self.state.keys_pressed.lock() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to lock keys_pressed mutex: {}", e);
                Vec::new()
            }
        }
    }

    pub fn add_callback(&self, callback: Callback) {
        self.state.add_callback(callback);
    }

    pub fn spawn(&self) -> thread::JoinHandle<()> {
        let state = Arc::clone(&self.state);

        thread::spawn(move || {
            if let Err(e) = listen(move |event| {
                match event.event_type {
                    EventType::MouseMove { x, y } => {
                        state.update_mouse_position(x as i32, y as i32);
                    }
                    EventType::ButtonPress(button) => {
                        state.update_button_press(button);
                    }
                    EventType::KeyPress(key) => {
                        state.update_key_press(key);
                    }
                    EventType::KeyRelease(key) => {
                        state.update_key_release(key);
                    }
                    EventType::ButtonRelease(_) => {
                        state.update_button_release();
                    }
                    _ => {}
                }

                // Execute all registered callbacks
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
