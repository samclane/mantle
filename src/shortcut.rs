use eframe::egui::{vec2, Response, Sense, Ui, Widget};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::sync::{Arc, Mutex};

use crate::action::UserAction;
use crate::listener::input_action::InputAction;
use crate::listener::input_listener::InputListener;
use crate::listener::key_mapping::from_egui;
use crate::LifxManager;

pub type ShortcutCallback = Arc<dyn Fn(InputAction) + Send + Sync + 'static>;

#[derive(Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct KeyboardShortcut {
    pub keys: InputAction,
    pub display_name: String,
}

impl KeyboardShortcut {
    pub fn new(keys: InputAction, display_name: String) -> Self {
        KeyboardShortcut { keys, display_name }
    }

    pub fn update_display_string(&mut self) {
        self.display_name = self
            .keys
            .iter()
            .map(|key| format!("{}", key))
            .collect::<Vec<_>>()
            .join(" + ");
    }

    fn is_matched(&self, keys_pressed: &InputAction) -> bool {
        self.keys.is_subset(keys_pressed)
    }
}

impl Default for KeyboardShortcut {
    fn default() -> Self {
        KeyboardShortcut {
            keys: InputAction::default(),
            display_name: "".to_string(),
        }
    }
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

#[derive(Clone, Serialize, Deserialize)]
pub struct KeyboardShortcutAction {
    pub shortcut: KeyboardShortcut,
    pub action: UserAction,
    pub name: String,
}

impl Default for KeyboardShortcutAction {
    fn default() -> Self {
        KeyboardShortcutAction {
            shortcut: KeyboardShortcut::default(),
            action: UserAction::Refresh,
            name: "".to_string(),
        }
    }
}

pub struct ShortcutManager {
    input_listener: InputListener,
    shortcuts: Arc<Mutex<Vec<KeyboardShortcutAction>>>,
    active_shortcuts: Arc<Mutex<BTreeSet<KeyboardShortcut>>>,
    pub new_shortcut: KeyboardShortcutAction,
}

impl ShortcutManager {
    pub fn new(input_listener: InputListener) -> Self {
        ShortcutManager {
            input_listener,
            shortcuts: Arc::new(Mutex::new(Vec::new())),
            active_shortcuts: Arc::new(Mutex::new(BTreeSet::new())),
            new_shortcut: KeyboardShortcutAction::default(),
        }
    }

    pub fn add_shortcut(
        &self,
        action_name: String,
        shortcut: KeyboardShortcut,
        action: UserAction,
    ) {
        let keyboard_shortcut_callback = KeyboardShortcutAction {
            shortcut: shortcut.clone(),
            action,
            name: action_name,
        };

        if let Ok(mut shortcuts) = self.shortcuts.lock() {
            shortcuts.push(keyboard_shortcut_callback);
        } else {
            log::error!("Failed to lock shortcuts mutex");
        }
    }

    pub fn get_active_shortcuts(&self) -> Vec<KeyboardShortcutAction> {
        if let Ok(shortcuts) = self.shortcuts.lock() {
            shortcuts.clone()
        } else {
            log::error!("Failed to lock shortcuts mutex");
            Vec::new()
        }
    }

    pub fn start(&self, lifx_manager: LifxManager) -> std::thread::JoinHandle<()> {
        let input_listener = self.input_listener.clone();
        let shortcuts: Arc<Mutex<Vec<KeyboardShortcutAction>>> = Arc::clone(&self.shortcuts);
        let active_shortcuts: Arc<Mutex<BTreeSet<KeyboardShortcut>>> =
            Arc::clone(&self.active_shortcuts);

        let input_listener_clone = input_listener.clone();
        self.input_listener.add_callback(Box::new(move |_event| {
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

            for shortcut_action in shortcuts_guard.iter() {
                if shortcut_action.shortcut.is_matched(&keys_pressed) {
                    if !active_shortcuts_guard.contains(&shortcut_action.shortcut) {
                        shortcut_action.action.execute(lifx_manager.clone());
                        active_shortcuts_guard.insert(shortcut_action.shortcut.clone());
                    }
                } else {
                    active_shortcuts_guard.remove(&shortcut_action.shortcut);
                }
            }
        }));

        self.input_listener.start()
    }
}

impl Default for ShortcutManager {
    fn default() -> Self {
        ShortcutManager {
            input_listener: InputListener::new(),
            shortcuts: Arc::new(Mutex::new(Vec::new())),
            active_shortcuts: Arc::new(Mutex::new(BTreeSet::new())),
            new_shortcut: KeyboardShortcutAction::default(),
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

        let desired_size = ui.spacing().interact_size * vec2(5.0, 1.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click());

        if response.clicked() {
            response.request_focus();
        }

        let is_focused = response.has_focus();

        let bg_color = if is_focused {
            ui.visuals().selection.bg_fill
        } else if response.hovered() {
            ui.visuals().widgets.hovered.bg_fill
        } else {
            ui.visuals().widgets.inactive.bg_fill
        };
        ui.painter().rect_filled(rect, 5.0, bg_color);

        let border_stroke = ui.visuals().widgets.active.bg_stroke;
        ui.painter().rect_stroke(rect, 5.0, border_stroke);

        if is_focused {
            ui.input(|inputstate: &eframe::egui::InputState| {
                let mut keys_pressed = InputAction::default();
                for key in inputstate.keys_down.iter() {
                    let modifiers = inputstate.modifiers;
                    let input_item = from_egui(*key, modifiers);
                    keys_pressed.extend(input_item);
                }
                shortcut.keys.extend(keys_pressed);
            });
        }
        shortcut.update_display_string();

        let text = shortcut.display_name.clone();
        let text_pos = rect.center();
        ui.painter().text(
            text_pos,
            eframe::egui::Align2::CENTER_CENTER,
            text,
            eframe::egui::TextStyle::Button.resolve(ui.style()),
            ui.visuals().text_color(),
        );

        response
    }
}

#[cfg(test)]
mod tests {
    use crate::listener::input_item::InputItem;

    use super::*;
    use rdev::Key;

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

        let keys_pressed = shortcut_keys.clone();
        assert!(shortcut.is_matched(&InputAction::from(keys_pressed.clone())));

        let mut keys_pressed_extra = keys_pressed.clone();
        keys_pressed_extra.insert(InputItem::Key(Key::ShiftLeft));
        assert!(shortcut.is_matched(&InputAction::from(keys_pressed_extra.clone())));

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
    fn test_shortcut_manager_add_shortcut() {
        let input_listener = InputListener::new();
        let shortcut_manager = ShortcutManager::new(input_listener);
        let keys: BTreeSet<_> = vec![InputItem::Key(Key::KeyA)].into_iter().collect();
        let shortcut =
            KeyboardShortcut::new(InputAction::from(keys.clone()), "TestAction".to_string());

        shortcut_manager.add_shortcut(
            "TestAction".to_string(),
            shortcut.clone(),
            UserAction::Refresh,
        );

        let shortcuts = shortcut_manager.get_active_shortcuts();
        assert_eq!(shortcuts.len(), 1);
        assert_eq!(shortcuts[0].shortcut, shortcut);
        assert_eq!(shortcuts[0].name, "TestAction");
    }
}
