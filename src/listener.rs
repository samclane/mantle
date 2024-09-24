use log::error;
use rdev::{listen, Button, Event, EventType, Key};
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::thread::{spawn, JoinHandle};
use std::time::Instant;

type BackgroundCallback = Box<dyn Fn(Event) + Send>;
type ShortcutCallback = Arc<dyn Fn(BTreeSet<InputAction>) + Send + Sync>;

#[derive(Clone, Copy, Debug)]
pub enum InputAction {
    Key(Key),
    Button(Button),
}

impl PartialEq for InputAction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (InputAction::Key(k1), InputAction::Key(k2)) => k1 == k2,
            (InputAction::Button(b1), InputAction::Button(b2)) => b1 == b2,
            _ => false,
        }
    }
}

impl Eq for InputAction {}

impl Hash for InputAction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            InputAction::Key(k) => k.hash(state),
            InputAction::Button(b) => b.hash(state),
        }
    }
}

impl Ord for InputAction {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (InputAction::Key(k1), InputAction::Key(k2)) => {
                format!("{:?}", k1).cmp(&format!("{:?}", k2))
            }
            (InputAction::Button(b1), InputAction::Button(b2)) => {
                format!("{:?}", b1).cmp(&format!("{:?}", b2))
            }
            (InputAction::Key(_), InputAction::Button(_)) => Ordering::Less,
            (InputAction::Button(_), InputAction::Key(_)) => Ordering::Greater,
        }
    }
}

impl PartialOrd for InputAction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for InputAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            InputAction::Key(k) => write!(f, "{:?}", k),
            InputAction::Button(b) => write!(f, "{:?}", b),
        }
    }
}

#[derive(Clone, Copy)]
pub struct MousePosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct KeyboardShortcut {
    pub keys: BTreeSet<InputAction>,
}

#[derive(Clone)]
pub struct KeyboardShortcutCallback {
    pub shortcut: KeyboardShortcut,
    pub callback: ShortcutCallback,
    pub callback_name: String,
}

impl Debug for KeyboardShortcut {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let keys: Vec<String> = self.keys.iter().map(|k| format!("{}", k)).collect();
        write!(f, "KeyboardShortcut({})", keys.join(" + "))
    }
}

impl Display for KeyboardShortcut {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let keys: Vec<String> = self.keys.iter().map(|k| format!("{}", k)).collect();
        write!(f, "{}", keys.join(" + "))
    }
}

impl KeyboardShortcut {
    fn is_matched(&self, keys_pressed: &BTreeSet<InputAction>) -> bool {
        self.keys.is_subset(keys_pressed)
    }
}

pub struct SharedInputState {
    last_mouse_position: Mutex<Option<MousePosition>>,
    last_click_time: Mutex<Option<Instant>>,
    keys_pressed: Mutex<BTreeSet<InputAction>>,
    callbacks: Mutex<Vec<BackgroundCallback>>,
    shortcuts: Mutex<Vec<KeyboardShortcutCallback>>,
    active_shortcuts: Mutex<BTreeSet<KeyboardShortcut>>,
}

impl SharedInputState {
    fn new() -> Self {
        SharedInputState {
            last_mouse_position: Mutex::new(None),
            last_click_time: Mutex::new(None),
            keys_pressed: Mutex::new(BTreeSet::new()),
            callbacks: Mutex::new(Vec::new()),
            shortcuts: Mutex::new(Vec::new()),
            active_shortcuts: Mutex::new(BTreeSet::new()),
        }
    }

    fn update_input_key_press(&self, input_key: InputAction) {
        if let Ok(mut keys) = self.keys_pressed.lock() {
            keys.insert(input_key);
        } else {
            error!("Failed to lock keys_pressed mutex");
        }
    }

    fn update_input_key_release(&self, input_key: InputAction) {
        if let Ok(mut keys) = self.keys_pressed.lock() {
            keys.remove(&input_key);
        } else {
            error!("Failed to lock keys_pressed mutex");
        }
    }

    fn update_mouse_position(&self, x: i32, y: i32) {
        if let Ok(mut pos) = self.last_mouse_position.lock() {
            *pos = Some(MousePosition { x, y });
        } else {
            error!("Failed to lock last_mouse_position mutex");
        }
    }

    fn update_button_press(&self, button: Button) {
        self.update_input_key_press(InputAction::Button(button));

        if let Ok(mut time) = self.last_click_time.lock() {
            *time = Some(Instant::now());
        } else {
            error!("Failed to lock last_click_time mutex");
        }
    }

    fn update_button_release(&self, button: Button) {
        self.update_input_key_release(InputAction::Button(button));
    }

    fn update_key_press(&self, key: Key) {
        self.update_input_key_press(InputAction::Key(key));
    }

    fn update_key_release(&self, key: Key) {
        self.update_input_key_release(InputAction::Key(key));
    }

    fn execute_callbacks(&self, event: &Event) {
        if let Ok(callbacks) = self.callbacks.lock() {
            for callback in callbacks.iter() {
                callback(event.clone());
            }
        } else {
            error!("Failed to lock callbacks mutex");
        }
    }

    fn add_callback(&self, callback: BackgroundCallback) {
        if let Ok(mut callbacks) = self.callbacks.lock() {
            callbacks.push(callback);
        } else {
            error!("Failed to lock callbacks mutex");
        }
    }

    fn add_shortcut_callback<F>(
        &self,
        shortcut: KeyboardShortcut,
        callback: F,
        callback_name: String,
    ) where
        F: Fn(BTreeSet<InputAction>) + Send + Sync + 'static,
    {
        let callback = Arc::new(callback);
        if let Ok(mut shortcuts) = self.shortcuts.lock() {
            shortcuts.push(KeyboardShortcutCallback {
                shortcut,
                callback,
                callback_name,
            });
        } else {
            error!("Failed to lock shortcuts mutex");
        }
    }

    fn check_shortcuts(&self) {
        let keys_pressed = match self.keys_pressed.lock() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to lock keys_pressed mutex: {}", e);
                BTreeSet::new()
            }
        };

        let shortcuts = match self.shortcuts.lock() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to lock shortcuts mutex: {}", e);
                Vec::new()
            }
        };

        let mut active_shortcuts = match self.active_shortcuts.lock() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to lock active_shortcuts mutex: {}", e);
                BTreeSet::new()
            }
        };

        for shortcut in &shortcuts {
            if shortcut.shortcut.is_matched(&keys_pressed) {
                if !active_shortcuts.contains(&shortcut.shortcut) {
                    // Shortcut is newly activated
                    (shortcut.callback)(keys_pressed.clone());

                    active_shortcuts.insert(shortcut.shortcut.clone());
                }
            } else {
                active_shortcuts.remove(&shortcut.shortcut);
            }
        }

        if let Ok(mut guard) = self.active_shortcuts.lock() {
            *guard = active_shortcuts;
        } else {
            error!("Failed to lock active_shortcuts mutex");
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

    pub fn is_input_key_pressed(&self, input_key: InputAction) -> bool {
        match self.state.keys_pressed.lock() {
            Ok(guard) => guard.contains(&input_key),
            Err(e) => {
                error!("Failed to lock keys_pressed mutex: {}", e);
                false
            }
        }
    }

    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.is_input_key_pressed(InputAction::Key(key))
    }

    pub fn is_button_pressed(&self, button: Button) -> bool {
        self.is_input_key_pressed(InputAction::Button(button))
    }

    pub fn get_keys_pressed(&self) -> BTreeSet<InputAction> {
        match self.state.keys_pressed.lock() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to lock keys_pressed mutex: {}", e);
                BTreeSet::new()
            }
        }
    }

    pub fn add_callback(&self, callback: BackgroundCallback) {
        self.state.add_callback(callback);
    }

    pub fn add_shortcut_callback<F>(
        &self,
        shortcut: KeyboardShortcut,
        callback: F,
        callback_name: String,
    ) where
        F: Fn(BTreeSet<InputAction>) + Send + Sync + 'static,
    {
        self.state
            .add_shortcut_callback(shortcut, callback, callback_name);
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

                // Execute all registered callbacks
                state.execute_callbacks(&event);

                // Check for keyboard shortcuts
                state.check_shortcuts();
            }) {
                error!("Error in listen: {:?}", e);
            }
        })
    }

    pub fn get_active_shortcuts(&self) -> Vec<KeyboardShortcutCallback> {
        match self.state.shortcuts.lock() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to lock shortcuts mutex: {}", e);
                Vec::new()
            }
        }
    }
}

impl Default for InputListener {
    fn default() -> Self {
        Self::new()
    }
}
