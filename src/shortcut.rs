use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use crate::listener::{
    InputAction, InputListener, KeyboardShortcut, KeyboardShortcutCallback, ShortcutCallback,
};

pub struct ShortcutManager {
    input_listener: InputListener,
    shortcuts: Arc<Mutex<HashMap<String, KeyboardShortcutCallback>>>,
}

impl ShortcutManager {
    pub fn new(
        input_listener: InputListener,
        shortcuts: Arc<Mutex<HashMap<String, KeyboardShortcutCallback>>>,
    ) -> Self {
        ShortcutManager {
            input_listener,
            shortcuts,
        }
    }

    pub fn add_shortcut<F>(
        &self,
        action_name: String,
        shortcut: KeyboardShortcut,
        callback: F,
        callback_name: String,
    ) where
        F: Fn(BTreeSet<InputAction>) + Send + Sync + 'static,
    {
        // Store the callback in an Arc
        let arc_callback: ShortcutCallback = Arc::new(callback);

        let keyboard_shortcut_callback = KeyboardShortcutCallback {
            shortcut: shortcut.clone(),
            callback: arc_callback.clone(),
            callback_name: action_name.clone(),
        };

        // Pass the closure directly to the InputListener
        self.input_listener
            .add_shortcut_callback(shortcut, arc_callback.clone(), callback_name);

        // Store the callback for future reference
        if let Ok(mut shortcuts) = self.shortcuts.lock() {
            shortcuts.insert(action_name, keyboard_shortcut_callback);
        } else {
            log::error!("Failed to lock shortcuts mutex");
        }
    }

    pub fn get_active_shortcuts(&self) -> Vec<KeyboardShortcutCallback> {
        if let Ok(shortcuts) = self.shortcuts.lock() {
            shortcuts.values().cloned().collect()
        } else {
            log::error!("Failed to lock shortcuts mutex");
            Vec::new()
        }
    }

    pub fn start(&self) -> std::thread::JoinHandle<()> {
        self.input_listener.start()
    }
}

impl Default for ShortcutManager {
    fn default() -> Self {
        ShortcutManager {
            input_listener: InputListener::new(),
            shortcuts: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}
