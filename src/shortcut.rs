// shortcut.rs
use crate::listener::{InputAction, InputListener};
use std::collections::BTreeSet;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::sync::{Arc, Mutex};

pub type ShortcutCallback = Arc<dyn Fn(BTreeSet<InputAction>) + Send + Sync + 'static>;

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct KeyboardShortcut {
    pub keys: BTreeSet<InputAction>,
}

impl KeyboardShortcut {
    pub fn new(keys: BTreeSet<InputAction>) -> Self {
        KeyboardShortcut { keys }
    }
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

pub struct ShortcutManager {
    input_listener: InputListener,
    shortcuts: Arc<Mutex<Vec<KeyboardShortcutCallback>>>,
    active_shortcuts: Arc<Mutex<BTreeSet<KeyboardShortcut>>>,
}

impl ShortcutManager {
    pub fn new(input_listener: InputListener) -> Self {
        ShortcutManager {
            input_listener,
            shortcuts: Arc::new(Mutex::new(Vec::new())),
            active_shortcuts: Arc::new(Mutex::new(BTreeSet::new())),
        }
    }

    pub fn add_shortcut<F>(&self, action_name: String, shortcut: KeyboardShortcut, callback: F)
    where
        F: Fn(BTreeSet<InputAction>) + Send + Sync + 'static,
    {
        let arc_callback: ShortcutCallback = Arc::new(callback);

        let keyboard_shortcut_callback = KeyboardShortcutCallback {
            shortcut: shortcut.clone(),
            callback: arc_callback.clone(),
            callback_name: action_name.clone(),
        };

        // Store the callback for future reference
        if let Ok(mut shortcuts) = self.shortcuts.lock() {
            shortcuts.push(keyboard_shortcut_callback);
        } else {
            log::error!("Failed to lock shortcuts mutex");
        }
    }

    pub fn get_active_shortcuts(&self) -> Vec<KeyboardShortcutCallback> {
        if let Ok(shortcuts) = self.shortcuts.lock() {
            shortcuts.clone()
        } else {
            log::error!("Failed to lock shortcuts mutex");
            Vec::new()
        }
    }

    pub fn start(&self) -> std::thread::JoinHandle<()> {
        let input_listener = self.input_listener.clone();
        let shortcuts: Arc<Mutex<Vec<KeyboardShortcutCallback>>> = Arc::clone(&self.shortcuts);
        let active_shortcuts: Arc<Mutex<BTreeSet<KeyboardShortcut>>> =
            Arc::clone(&self.active_shortcuts);

        // Register a background callback with the InputListener
        let input_listener_clone = input_listener.clone();
        self.input_listener.add_callback(Box::new(move |_event| {
            let keys_pressed = input_listener_clone.get_keys_pressed();

            let mut active_shortcuts_guard = match active_shortcuts.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    log::error!("Failed to lock active_shortcuts mutex: {}", e);
                    return;
                }
            };

            let shortcuts_guard = match shortcuts.lock() {
                Ok(guard) => guard,
                Err(e) => {
                    log::error!("Failed to lock shortcuts mutex: {}", e);
                    return;
                }
            };

            for shortcut_callback in shortcuts_guard.iter() {
                if shortcut_callback.shortcut.is_matched(&keys_pressed) {
                    if !active_shortcuts_guard.contains(&shortcut_callback.shortcut) {
                        // Shortcut is newly activated
                        (shortcut_callback.callback)(keys_pressed.clone());
                        active_shortcuts_guard.insert(shortcut_callback.shortcut.clone());
                    }
                } else {
                    active_shortcuts_guard.remove(&shortcut_callback.shortcut);
                }
            }
        }));

        // Start the InputListener
        self.input_listener.start()
    }
}

impl Default for ShortcutManager {
    fn default() -> Self {
        ShortcutManager {
            input_listener: InputListener::new(),
            shortcuts: Arc::new(Mutex::new(Vec::new())),
            active_shortcuts: Arc::new(Mutex::new(BTreeSet::new())),
        }
    }
}
