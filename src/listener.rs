use rdev::{listen, Event, EventType};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

type Callback = Box<dyn Fn(Event) + Send>;

pub struct SharedInputState {
    last_mouse_position: Mutex<Option<(i32, i32)>>,
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
        if let Ok(mut pos) = self.last_mouse_position.lock() {
            *pos = Some((x, y));
        }
    }

    fn update_button_press(&self, button: rdev::Button) {
        if let Ok(mut pressed) = self.button_pressed.lock() {
            *pressed = Some(button);
        }
        if let Ok(mut time) = self.last_click_time.lock() {
            *time = Some(Instant::now());
        }
        if let Ok(mut last) = self.last_button_pressed.lock() {
            *last = Some(button);
        }
    }

    fn update_key_press(&self, key: rdev::Key) {
        if let Ok(mut keys) = self.keys_pressed.lock() {
            keys.push(key);
            if let Ok(mut last) = self.last_keys_pressed.lock() {
                *last = keys.clone();
            }
        }
    }

    fn update_key_release(&self, key: rdev::Key) {
        if let Ok(mut keys) = self.keys_pressed.lock() {
            keys.retain(|&k| k != key);
        }
    }

    fn update_button_release(&self) {
        if let Ok(mut pressed) = self.button_pressed.lock() {
            *pressed = None;
        }
    }

    fn execute_callbacks(&self, event: &Event) {
        if let Ok(callbacks) = self.callbacks.lock() {
            for callback in callbacks.iter() {
                callback(event.clone());
            }
        }
    }

    fn add_callback(&self, callback: Callback) {
        if let Ok(mut callbacks) = self.callbacks.lock() {
            callbacks.push(callback);
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

    pub fn get_last_mouse_position(&self) -> Option<(i32, i32)> {
        *self.state.last_mouse_position.lock().unwrap()
    }

    pub fn get_last_click_time(&self) -> Option<Instant> {
        *self.state.last_click_time.lock().unwrap()
    }

    pub fn get_last_button_pressed(&self) -> Option<rdev::Button> {
        *self.state.last_button_pressed.lock().unwrap()
    }

    pub fn is_button_pressed(&self, button: rdev::Button) -> bool {
        match *self.state.button_pressed.lock().unwrap() {
            Some(b) => b == button,
            None => false,
        }
    }

    pub fn is_key_pressed(&self, key: rdev::Key) -> bool {
        self.state.keys_pressed.lock().unwrap().contains(&key)
    }

    pub fn get_keys_pressed(&self) -> Vec<rdev::Key> {
        self.state.keys_pressed.lock().unwrap().clone()
    }

    pub fn add_callback(&self, callback: Callback) {
        self.state.add_callback(callback);
    }

    pub fn spawn(&self) -> thread::JoinHandle<()> {
        let state = Arc::clone(&self.state);

        thread::spawn(move || {
            listen(move |event| {
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
            })
            .expect("Could not listen");
        })
    }
}

impl Default for InputListener {
    fn default() -> Self {
        Self::new()
    }
}
