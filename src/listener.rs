use log::error;
use rdev::{listen, Button, Event, EventType, Key};
use std::collections::HashSet;
use std::fmt::{Debug, Display, Formatter, Result};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::thread::{spawn, JoinHandle};
use std::time::Instant;

type BackgroundCallback = Box<dyn Fn(Event) + Send>;
type ShortcutCallback = Arc<dyn Fn(HashSet<Key>) + Send + Sync>;

#[derive(Clone, Copy)]
pub struct MousePosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Eq, PartialEq)]
pub struct KeyboardShortcut {
    pub keys: HashSet<Key>,
}

#[derive(Clone)]
pub struct KeyboardShortcutCallback {
    pub shortcut: KeyboardShortcut,
    pub callback: ShortcutCallback,
    pub name: String,
    pub callback_name: String,
}

impl Hash for KeyboardShortcut {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for key in &self.keys {
            key.hash(state);
        }
    }
}

impl Debug for KeyboardShortcut {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "KeyboardShortcut({:?})", self.keys)
    }
}

impl Display for KeyboardShortcut {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let keys: Vec<String> = self.keys.iter().map(|k| format!("{:?}", k)).collect();
        write!(f, "{}", keys.join(" + "))
    }
}

impl KeyboardShortcut {
    fn is_matched(&self, keys_pressed: &HashSet<Key>) -> bool {
        self.keys.is_subset(keys_pressed)
    }
}

pub struct SharedInputState {
    last_mouse_position: Mutex<Option<MousePosition>>,
    last_click_time: Mutex<Option<Instant>>,
    button_pressed: Mutex<Option<Button>>,
    last_button_pressed: Mutex<Option<Button>>,
    keys_pressed: Mutex<HashSet<Key>>,
    last_keys_pressed: Mutex<HashSet<Key>>,
    callbacks: Mutex<Vec<BackgroundCallback>>,
    shortcuts: Mutex<Vec<KeyboardShortcutCallback>>,
    active_shortcuts: Mutex<HashSet<KeyboardShortcut>>,
}

impl SharedInputState {
    fn new() -> Self {
        SharedInputState {
            last_mouse_position: Mutex::new(None),
            last_click_time: Mutex::new(None),
            button_pressed: Mutex::new(None),
            last_button_pressed: Mutex::new(None),
            keys_pressed: Mutex::new(HashSet::new()),
            last_keys_pressed: Mutex::new(HashSet::new()),
            callbacks: Mutex::new(Vec::new()),
            shortcuts: Mutex::new(Vec::new()),
            active_shortcuts: Mutex::new(HashSet::new()),
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

    fn update_button_press(&self, button: Button) {
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

    fn update_key_press(&self, key: Key) {
        match self.keys_pressed.lock() {
            Ok(mut keys) => {
                keys.insert(key);

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

    fn update_key_release(&self, key: Key) {
        match self.keys_pressed.lock() {
            Ok(mut keys) => {
                keys.remove(&key);
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

    fn add_callback(&self, callback: BackgroundCallback) {
        match self.callbacks.lock() {
            Ok(mut callbacks) => {
                callbacks.push(callback);
            }
            Err(e) => {
                error!("Failed to lock callbacks mutex: {}", e);
            }
        }
    }

    fn add_shortcut_callback<F>(&self, shortcut: KeyboardShortcut, callback: F)
    where
        F: Fn(HashSet<Key>) + Send + Sync + 'static,
    {
        let callback = Arc::new(callback);
        match self.shortcuts.lock() {
            Ok(mut shortcuts) => {
                let keys = shortcut
                    .keys
                    .iter()
                    .map(|k| format!("{:?}", k))
                    .collect::<Vec<_>>();
                let name = format!("{}", shortcut);
                shortcuts.push(KeyboardShortcutCallback {
                    shortcut,
                    callback,
                    name,
                    callback_name: format!("{:?}", keys),
                });
            }
            Err(e) => {
                error!("Failed to lock shortcuts mutex: {}", e);
            }
        }
    }

    fn check_shortcuts(&self) {
        let keys_pressed = match self.keys_pressed.lock() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to lock keys_pressed mutex: {}", e);
                HashSet::new()
            }
        };

        let shortcuts = match self.shortcuts.lock() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to lock shortcuts mutex: {}", e);
                Vec::new()
            }
        };

        let active_shortcuts = match self.active_shortcuts.lock() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to lock active_shortcuts mutex: {}", e);
                HashSet::new()
            }
        };

        for shortcut in &shortcuts {
            if shortcut.shortcut.is_matched(&keys_pressed) {
                if !active_shortcuts.contains(&shortcut.shortcut) {
                    // Shortcut is newly activated
                    (shortcut.callback)(keys_pressed.clone());

                    // Add to active_shortcuts
                    if let Ok(mut guard) = self.active_shortcuts.lock() {
                        guard.insert(shortcut.shortcut.clone());
                    } else {
                        error!("Failed to lock active_shortcuts mutex");
                    }
                }
            } else {
                // Remove from active_shortcuts if present
                if let Ok(mut guard) = self.active_shortcuts.lock() {
                    guard.remove(&shortcut.shortcut);
                } else {
                    error!("Failed to lock active_shortcuts mutex");
                }
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

    pub fn get_last_button_pressed(&self) -> Option<Button> {
        match self.state.last_button_pressed.lock() {
            Ok(guard) => *guard,
            Err(e) => {
                error!("Failed to lock last_button_pressed mutex: {}", e);
                None
            }
        }
    }

    pub fn is_button_pressed(&self, button: Button) -> bool {
        match self.state.button_pressed.lock() {
            Ok(guard) => guard.map_or(false, |b| b == button),
            Err(e) => {
                error!("Failed to lock button_pressed mutex: {}", e);
                false
            }
        }
    }

    pub fn is_key_pressed(&self, key: Key) -> bool {
        match self.state.keys_pressed.lock() {
            Ok(guard) => guard.contains(&key),
            Err(e) => {
                error!("Failed to lock keys_pressed mutex: {}", e);
                false
            }
        }
    }

    pub fn get_keys_pressed(&self) -> HashSet<Key> {
        match self.state.keys_pressed.lock() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to lock keys_pressed mutex: {}", e);
                HashSet::new()
            }
        }
    }

    pub fn add_callback(&self, callback: BackgroundCallback) {
        self.state.add_callback(callback);
    }

    pub fn add_shortcut_callback<F>(&self, shortcut: KeyboardShortcut, callback: F)
    where
        F: Fn(HashSet<Key>) + Send + Sync + 'static,
    {
        self.state.add_shortcut_callback(shortcut, callback);
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

                // Check for keyboard shortcuts
                state.check_shortcuts();
            }) {
                error!("Error in listen: {:?}", e);
            }
        })
    }

    pub fn get_active_items(&self) -> impl Iterator<Item = KeyboardShortcutCallback> {
        self.state
            .shortcuts
            .lock()
            .unwrap()
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl Default for InputListener {
    fn default() -> Self {
        Self::new()
    }
}
