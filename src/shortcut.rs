use crate::listener::{from_egui, InputAction, InputListener};
use eframe::egui::{vec2, Response, Sense, TextBuffer, Ui, Widget};
use std::collections::BTreeSet;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

pub type ShortcutCallback = Arc<dyn Fn(InputAction) + Send + Sync + 'static>;

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct KeyboardShortcut {
    pub keys: InputAction,
    display_name: String,
}

impl KeyboardShortcut {
    pub fn new(keys: InputAction, display_name: String) -> Self {
        KeyboardShortcut { keys, display_name }
    }

    fn update_display_string(&mut self) {
        self.display_name = self
            .keys
            .0
            .iter()
            .fold(String::new(), |acc, key| acc + &format!("{} + ", key));
    }

    fn is_matched(&self, keys_pressed: &InputAction) -> bool {
        self.keys.0.is_subset(&keys_pressed.0)
    }
}

impl TextBuffer for KeyboardShortcut {
    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        &self.display_name
    }

    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        let mut new_keys = self.keys.clone();
        for (offset, c) in text.chars().enumerate() {
            if offset != char_index {
                continue;
            }
            if let Ok(keys) = InputAction::from_str(&c.to_string()) {
                // combine the 2 sets
                new_keys.0.extend(keys.0);
            }
        }
        self.keys = new_keys;
        self.update_display_string();
        char_index + text.chars().count()
    }

    fn delete_char_range(&mut self, char_range: std::ops::Range<usize>) {
        let keys_vec: Vec<_> = self.keys.0.iter().cloned().collect();
        for i in char_range {
            if let Some(key) = keys_vec.get(i) {
                self.keys.0.remove(key);
            }
        }
        self.update_display_string();
    }
}

impl Debug for KeyboardShortcut {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let keys: Vec<String> = self.keys.0.iter().map(|k| format!("{}", k)).collect();
        write!(f, "KeyboardShortcut({})", keys.join(" + "))
    }
}

impl Display for KeyboardShortcut {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let keys: Vec<String> = self.keys.0.iter().map(|k| format!("{}", k)).collect();
        write!(f, "{}", keys.join(" + "))
    }
}

#[derive(Clone)]
pub struct KeyboardShortcutCallback {
    pub shortcut: KeyboardShortcut,
    pub callback: ShortcutCallback,
    pub callback_name: String,
}

pub struct ShortcutManager {
    input_listener: InputListener,
    shortcuts: Arc<Mutex<Vec<KeyboardShortcutCallback>>>,
    active_shortcuts: Arc<Mutex<BTreeSet<KeyboardShortcut>>>,
    pub new_shortcut: KeyboardShortcutCallback,
}

impl ShortcutManager {
    pub fn new(input_listener: InputListener) -> Self {
        ShortcutManager {
            input_listener,
            shortcuts: Arc::new(Mutex::new(Vec::new())),
            active_shortcuts: Arc::new(Mutex::new(BTreeSet::new())),
            new_shortcut: KeyboardShortcutCallback {
                shortcut: KeyboardShortcut {
                    keys: InputAction::default(),
                    display_name: "".to_string(),
                },
                callback: Arc::new(|_keys_pressed| {}),
                callback_name: "".to_string(),
            },
        }
    }

    pub fn add_shortcut<F>(&self, action_name: String, shortcut: KeyboardShortcut, callback: F)
    where
        F: Fn(InputAction) + Send + Sync + 'static,
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
            // cast keys_pressed to InputAction
            let keys_pressed = InputAction::from(input_listener_clone.get_keys_pressed());

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
            new_shortcut: KeyboardShortcutCallback {
                shortcut: KeyboardShortcut {
                    keys: InputAction::default(),
                    display_name: "".to_string(),
                },
                callback: Arc::new(|_keys_pressed| {}),
                callback_name: "".to_string(),
            },
        }
    }
}

pub struct ShortcutEdit<'a> {
    shortcut: &'a mut KeyboardShortcut,
}

impl<'a> ShortcutEdit<'a> {
    pub fn new(shortcut: &'a mut KeyboardShortcut) -> Self {
        Self { shortcut }
    }
}

impl<'a> Widget for ShortcutEdit<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let ShortcutEdit { shortcut } = self;

        // Allocate space for the widget
        let desired_size = ui.spacing().interact_size.y * vec2(1.0, 1.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        // Handle focus
        if response.clicked() {
            response.request_focus();
        }

        let is_focused = response.has_focus();

        // Draw background
        let bg_color = if is_focused {
            ui.visuals().selection.bg_fill
        } else {
            ui.visuals().widgets.inactive.bg_fill
        };
        ui.painter().rect_filled(rect, 0.0, bg_color);

        // Handle key input when focused
        if is_focused {
            ui.input(|inputstate: &eframe::egui::InputState| {
                let mut keys_pressed = InputAction::default();
                for key in inputstate.keys_down.iter() {
                    let modifiers = inputstate.modifiers;
                    let input_item = from_egui(*key, modifiers);
                    keys_pressed.0.extend(input_item.0);
                }
                shortcut.keys = keys_pressed;
                shortcut.update_display_string();
            });
        }

        // Display the current shortcut
        // let text_style = TextStyle::Button;
        // let galley = ui.fonts(|_|{}).layout_single_line(
        //     text_style.resolve(ui.style()),
        //     shortcut.display_string.clone(),
        // );
        // let text_pos = rect.center() - galley.size() * 0.5;
        // ui.painter().galley(text_pos, galley);
        let text = shortcut.display_name.clone();
        ui.label(text);

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listener::{InputAction, InputItem, InputListener};
    use rdev::Key;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn test_keyboard_shortcut_new() {
        let keys: BTreeSet<_> = vec![InputItem::Key(Key::KeyA)].into_iter().collect();
        let shortcut =
            KeyboardShortcut::new(InputAction::from(keys.clone()), "TestAction".to_string());
        assert_eq!(shortcut.keys, InputAction::from(keys.clone()));
    }

    #[test]
    fn test_keyboard_shortcut_is_matched() {
        let shortcut_keys: BTreeSet<_> =
            vec![InputItem::Key(Key::ControlLeft), InputItem::Key(Key::KeyA)]
                .into_iter()
                .collect();
        let shortcut = KeyboardShortcut::new(
            InputAction::from(shortcut_keys.clone()),
            "TestAction".to_string(),
        );

        // Test with matching keys_pressed
        let keys_pressed = shortcut_keys.clone();
        assert!(shortcut.is_matched(&InputAction::from(keys_pressed.clone())));

        // Test with extra keys in keys_pressed
        let mut keys_pressed_extra = keys_pressed.clone();
        keys_pressed_extra.insert(InputItem::Key(Key::ShiftLeft));
        assert!(shortcut.is_matched(&InputAction::from(keys_pressed_extra.clone())));

        // Test with missing keys in keys_pressed
        let keys_pressed_missing: BTreeSet<_> =
            vec![InputItem::Key(Key::ControlLeft)].into_iter().collect();
        assert!(!shortcut.is_matched(&InputAction::from(keys_pressed_missing.clone())));
    }

    #[test]
    fn test_keyboard_shortcut_display() {
        let keys: BTreeSet<_> = vec![InputItem::Key(Key::ControlLeft), InputItem::Key(Key::KeyA)]
            .into_iter()
            .collect();
        let shortcut =
            KeyboardShortcut::new(InputAction::from(keys.clone()), "TestAction".to_string());
        let display_str = format!("{}", shortcut);
        assert!(display_str.contains("ControlLeft"));
        assert!(display_str.contains("KeyA"));
    }

    #[test]
    fn test_keyboard_shortcut_callback_creation() {
        let keys: BTreeSet<_> = vec![InputItem::Key(Key::KeyA)].into_iter().collect();
        let shortcut =
            KeyboardShortcut::new(InputAction::from(keys.clone()), "TestAction".to_string());
        let callback_called = Arc::new(AtomicBool::new(false));
        let callback_called_clone = Arc::clone(&callback_called);

        let callback: ShortcutCallback = Arc::new(move |_keys_pressed| {
            callback_called_clone.store(true, Ordering::SeqCst);
        });

        let shortcut_callback = KeyboardShortcutCallback {
            shortcut,
            callback,
            callback_name: "TestAction".to_string(),
        };

        // Simulate calling the callback
        (shortcut_callback.callback)(InputAction::from(keys.clone()));
        assert!(callback_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_shortcut_manager_add_shortcut() {
        let input_listener = InputListener::new();
        let shortcut_manager = ShortcutManager::new(input_listener);
        let keys: BTreeSet<_> = vec![InputItem::Key(Key::KeyA)].into_iter().collect();
        let shortcut =
            KeyboardShortcut::new(InputAction::from(keys.clone()), "TestAction".to_string());

        shortcut_manager.add_shortcut(
            "TestAction".to_string(),
            shortcut.clone(),
            |_keys_pressed| {},
        );

        let shortcuts = shortcut_manager.get_active_shortcuts();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].shortcut, shortcut);
        assert_eq!(shortcuts[0].callback_name, "TestAction");
    }
}
