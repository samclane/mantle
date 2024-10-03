use std::{
    collections::BTreeSet,
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
    sync::{Arc, Mutex},
    thread::{spawn, JoinHandle},
    time::Instant,
};

use eframe::egui;
use log::error;
use rdev::{listen, Button, Event, EventType, Key};

pub type BackgroundCallback = Box<dyn Fn(Event) + Send>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputItem {
    Key(Key),
    Button(Button),
}

impl Display for InputItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            InputItem::Key(k) => write!(f, "{:?}", k),
            InputItem::Button(b) => write!(f, "{:?}", b),
        }
    }
}

#[derive(Debug)]
pub enum InputItemParseError {
    InvalidInput(String),
}

impl Display for InputItemParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "{}",
            match self {
                InputItemParseError::InvalidInput(s) => s,
            }
        )
    }
}

impl std::error::Error for InputItemParseError {}

impl FromStr for InputItem {
    type Err = InputItemParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s_lower = s.to_ascii_lowercase();
        match s_lower.as_str() {
            "ctrl" => Ok(InputItem::Key(Key::ControlLeft)),
            "alt" => Ok(InputItem::Key(Key::Alt)),
            "shift" => Ok(InputItem::Key(Key::ShiftLeft)),
            "cmd" | "meta" => Ok(InputItem::Key(Key::MetaLeft)),
            "left" => Ok(InputItem::Button(Button::Left)),
            "right" => Ok(InputItem::Button(Button::Right)),
            "middle" => Ok(InputItem::Button(Button::Middle)),
            "space" => Ok(InputItem::Key(Key::Space)),
            "enter" | "return" => Ok(InputItem::Key(Key::Return)),
            "escape" => Ok(InputItem::Key(Key::Escape)),
            "tab" => Ok(InputItem::Key(Key::Tab)),
            s if s.len() == 1 && s.chars().all(|c| c.is_ascii_alphabetic()) => {
                let c = s.chars().next().unwrap().to_ascii_uppercase();
                let key = match c {
                    'A' => Key::KeyA,
                    'B' => Key::KeyB,
                    'C' => Key::KeyC,
                    'D' => Key::KeyD,
                    'E' => Key::KeyE,
                    'F' => Key::KeyF,
                    'G' => Key::KeyG,
                    'H' => Key::KeyH,
                    'I' => Key::KeyI,
                    'J' => Key::KeyJ,
                    'K' => Key::KeyK,
                    'L' => Key::KeyL,
                    'M' => Key::KeyM,
                    'N' => Key::KeyN,
                    'O' => Key::KeyO,
                    'P' => Key::KeyP,
                    'Q' => Key::KeyQ,
                    'R' => Key::KeyR,
                    'S' => Key::KeyS,
                    'T' => Key::KeyT,
                    'U' => Key::KeyU,
                    'V' => Key::KeyV,
                    'W' => Key::KeyW,
                    'X' => Key::KeyX,
                    'Y' => Key::KeyY,
                    'Z' => Key::KeyZ,
                    _ => unreachable!(),
                };
                Ok(InputItem::Key(key))
            }
            _ => Err(InputItemParseError::InvalidInput(s.to_string())),
        }
    }
}

impl PartialOrd for InputItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for InputItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;

        match (self, other) {
            (InputItem::Key(k1), InputItem::Key(k2)) => {
                format!("{:?}", k1).cmp(&format!("{:?}", k2))
            }
            (InputItem::Button(b1), InputItem::Button(b2)) => {
                format!("{:?}", b1).cmp(&format!("{:?}", b2))
            }
            (InputItem::Key(_), InputItem::Button(_)) => Ordering::Less,
            (InputItem::Button(_), InputItem::Key(_)) => Ordering::Greater,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct InputAction(pub BTreeSet<InputItem>);

impl Display for InputAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut items: Vec<String> = self.0.iter().map(|item| item.to_string()).collect();
        items.sort();
        write!(f, "{}", items.join("+"))
    }
}

impl From<BTreeSet<InputItem>> for InputAction {
    fn from(items: BTreeSet<InputItem>) -> Self {
        InputAction(items)
    }
}

#[derive(Debug)]
pub enum InputActionParseError {
    InvalidItem(String),
}

impl Display for InputActionParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(
            f,
            "{}",
            match self {
                InputActionParseError::InvalidItem(s) => s,
            }
        )
    }
}

impl std::error::Error for InputActionParseError {}

impl FromStr for InputAction {
    type Err = InputActionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split('+');
        let mut items = BTreeSet::new();

        for part in parts {
            let part = part.trim();
            match InputItem::from_str(part) {
                Ok(item) => {
                    items.insert(item);
                }
                Err(_) => {
                    return Err(InputActionParseError::InvalidItem(part.to_string()));
                }
            }
        }

        Ok(InputAction(items))
    }
}

#[derive(Debug)]
pub struct KeyMappingError {
    key: egui::Key,
}

impl Display for KeyMappingError {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "Failed to map egui::Key to rdev::Key: {:?}", self.key)
    }
}

impl std::error::Error for KeyMappingError {}

pub fn map_egui_key_to_rdev_key(key: egui::Key) -> Result<Key, KeyMappingError> {
    match key {
        egui::Key::ArrowDown => Ok(Key::DownArrow),
        egui::Key::ArrowLeft => Ok(Key::LeftArrow),
        egui::Key::ArrowRight => Ok(Key::RightArrow),
        egui::Key::ArrowUp => Ok(Key::UpArrow),
        egui::Key::Backspace => Ok(Key::Backspace),
        egui::Key::Delete => Ok(Key::Delete),
        egui::Key::End => Ok(Key::End),
        egui::Key::Enter => Ok(Key::Return),
        egui::Key::Escape => Ok(Key::Escape),
        egui::Key::Home => Ok(Key::Home),
        egui::Key::Insert => Ok(Key::Insert),
        egui::Key::PageDown => Ok(Key::PageDown),
        egui::Key::PageUp => Ok(Key::PageUp),
        egui::Key::Space => Ok(Key::Space),
        egui::Key::Tab => Ok(Key::Tab),
        egui::Key::A => Ok(Key::KeyA),
        egui::Key::B => Ok(Key::KeyB),
        egui::Key::C => Ok(Key::KeyC),
        egui::Key::D => Ok(Key::KeyD),
        egui::Key::E => Ok(Key::KeyE),
        egui::Key::F => Ok(Key::KeyF),
        egui::Key::G => Ok(Key::KeyG),
        egui::Key::H => Ok(Key::KeyH),
        egui::Key::I => Ok(Key::KeyI),
        egui::Key::J => Ok(Key::KeyJ),
        egui::Key::K => Ok(Key::KeyK),
        egui::Key::L => Ok(Key::KeyL),
        egui::Key::M => Ok(Key::KeyM),
        egui::Key::N => Ok(Key::KeyN),
        egui::Key::O => Ok(Key::KeyO),
        egui::Key::P => Ok(Key::KeyP),
        egui::Key::Q => Ok(Key::KeyQ),
        egui::Key::R => Ok(Key::KeyR),
        egui::Key::S => Ok(Key::KeyS),
        egui::Key::T => Ok(Key::KeyT),
        egui::Key::U => Ok(Key::KeyU),
        egui::Key::V => Ok(Key::KeyV),
        egui::Key::W => Ok(Key::KeyW),
        egui::Key::X => Ok(Key::KeyX),
        egui::Key::Y => Ok(Key::KeyY),
        egui::Key::Z => Ok(Key::KeyZ),
        _ => Err(KeyMappingError { key }),
    }
}

pub fn from_egui(key: egui::Key, modifiers: egui::Modifiers) -> InputAction {
    let mut items = BTreeSet::new();

    if modifiers.ctrl {
        items.insert(InputItem::Key(Key::ControlLeft));
    }
    if modifiers.alt {
        items.insert(InputItem::Key(Key::Alt));
    }
    if modifiers.shift {
        items.insert(InputItem::Key(Key::ShiftLeft));
    }
    if modifiers.command {
        items.insert(InputItem::Key(Key::MetaLeft));
    }

    match map_egui_key_to_rdev_key(key) {
        Ok(rdev_key) => {
            items.insert(InputItem::Key(rdev_key));
        }
        Err(err) => {
            error!("{}", err);
        }
    }

    InputAction(items)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MousePosition {
    pub x: i32,
    pub y: i32,
}

pub struct SharedInputState {
    last_mouse_position: Mutex<Option<MousePosition>>,
    last_click_time: Mutex<Option<Instant>>,
    keys_pressed: Mutex<BTreeSet<InputItem>>,
    callbacks: Mutex<Vec<BackgroundCallback>>,
}

impl SharedInputState {
    fn new() -> Self {
        SharedInputState {
            last_mouse_position: Mutex::new(None),
            last_click_time: Mutex::new(None),
            keys_pressed: Mutex::new(BTreeSet::new()),
            callbacks: Mutex::new(Vec::new()),
        }
    }

    fn update_input_key_press(&self, input_key: InputItem) {
        let mut keys = self
            .keys_pressed
            .lock()
            .expect("Failed to lock keys_pressed mutex");
        keys.insert(input_key);
    }

    fn update_input_key_release(&self, input_key: InputItem) {
        let mut keys = self
            .keys_pressed
            .lock()
            .expect("Failed to lock keys_pressed mutex");
        keys.remove(&input_key);
    }

    fn update_mouse_position(&self, x: i32, y: i32) {
        let mut pos = self
            .last_mouse_position
            .lock()
            .expect("Failed to lock last_mouse_position mutex");
        *pos = Some(MousePosition { x, y });
    }

    fn update_button_press(&self, button: Button) {
        self.update_input_key_press(InputItem::Button(button));

        let mut time = self
            .last_click_time
            .lock()
            .expect("Failed to lock last_click_time mutex");
        *time = Some(Instant::now());
    }

    fn update_button_release(&self, button: Button) {
        self.update_input_key_release(InputItem::Button(button));
    }

    fn update_key_press(&self, key: Key) {
        self.update_input_key_press(InputItem::Key(key));
    }

    fn update_key_release(&self, key: Key) {
        self.update_input_key_release(InputItem::Key(key));
    }

    fn execute_callbacks(&self, event: &Event) {
        let callbacks = self
            .callbacks
            .lock()
            .expect("Failed to lock callbacks mutex");
        for callback in callbacks.iter() {
            callback(event.clone());
        }
    }
}

#[derive(Clone)]
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
        input_action.0.is_subset(&keys)
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

#[cfg(test)]
mod tests {
    use std::{
        hash::{Hash, Hasher},
        time::{Duration, SystemTime},
    };

    use super::*;
    use rdev::{Button, Event, EventType, Key};

    #[test]
    fn test_input_item_from_str() {
        assert_eq!(
            InputItem::from_str("ctrl").unwrap(),
            InputItem::Key(Key::ControlLeft)
        );
        assert_eq!(
            InputItem::from_str("Shift").unwrap(),
            InputItem::Key(Key::ShiftLeft)
        );
        assert_eq!(InputItem::from_str("a").unwrap(), InputItem::Key(Key::KeyA));
        assert_eq!(
            InputItem::from_str("Left").unwrap(),
            InputItem::Button(Button::Left)
        );
        assert!(InputItem::from_str("invalid").is_err());
    }

    #[test]
    fn test_input_action_from_str() {
        let action = InputAction::from_str("ctrl+alt+a").unwrap();
        let expected_items: BTreeSet<_> = vec![
            InputItem::Key(Key::ControlLeft),
            InputItem::Key(Key::Alt),
            InputItem::Key(Key::KeyA),
        ]
        .into_iter()
        .collect();
        assert_eq!(action.0, expected_items);

        assert!(InputAction::from_str("ctrl+invalid").is_err());
    }

    #[test]
    fn test_map_egui_key_to_rdev_key() {
        assert_eq!(map_egui_key_to_rdev_key(egui::Key::A).unwrap(), Key::KeyA);
        assert_eq!(
            map_egui_key_to_rdev_key(egui::Key::Enter).unwrap(),
            Key::Return
        );
        assert!(map_egui_key_to_rdev_key(egui::Key::F1).is_err());
    }

    #[test]
    fn test_from_egui() {
        let modifiers = egui::Modifiers {
            alt: true,
            ctrl: true,
            shift: false,
            mac_cmd: false,
            command: false,
        };
        let input_action = from_egui(egui::Key::A, modifiers);
        let expected_items: BTreeSet<_> = vec![
            InputItem::Key(Key::Alt),
            InputItem::Key(Key::ControlLeft),
            InputItem::Key(Key::KeyA),
        ]
        .into_iter()
        .collect();
        assert_eq!(input_action.0, expected_items);
    }

    #[test]
    fn test_is_input_action_pressed() {
        let listener = InputListener::new();
        listener.state.update_key_press(Key::ControlLeft);
        listener.state.update_key_press(Key::KeyA);

        let action = InputAction::from_str("ctrl+a").unwrap();
        assert!(listener.is_input_action_pressed(&action));

        listener.state.update_key_release(Key::ControlLeft);
        assert!(!listener.is_input_action_pressed(&action));
    }

    #[test]
    fn test_input_action_equality() {
        let key_a = InputItem::Key(Key::KeyA);
        let key_a2 = InputItem::Key(Key::KeyA);
        let key_b = InputItem::Key(Key::KeyB);
        let button_left = InputItem::Button(Button::Left);
        let button_right = InputItem::Button(Button::Right);

        assert_eq!(key_a, key_a2);
        assert_ne!(key_a, key_b);
        assert_ne!(key_a, button_left);
        assert_ne!(button_left, button_right);
    }

    #[test]
    fn test_input_action_ordering() {
        let mut actions = vec![
            InputItem::Key(Key::KeyB),
            InputItem::Button(Button::Left),
            InputItem::Key(Key::KeyA),
            InputItem::Button(Button::Right),
        ];

        actions.sort();

        assert_eq!(
            actions,
            vec![
                InputItem::Key(Key::KeyA),
                InputItem::Key(Key::KeyB),
                InputItem::Button(Button::Left),
                InputItem::Button(Button::Right),
            ]
        );
    }

    #[test]
    fn test_input_action_display() {
        let key = InputItem::Key(Key::KeyA);
        let button = InputItem::Button(Button::Left);

        assert_eq!(format!("{}", key), "KeyA");
        assert_eq!(format!("{}", button), "Left");
    }

    #[test]
    fn test_input_action_hash() {
        use std::collections::hash_map::DefaultHasher;

        let key_a1 = InputItem::Key(Key::KeyA);
        let key_a2 = InputItem::Key(Key::KeyA);
        let button_left = InputItem::Button(Button::Left);

        let mut hasher1 = DefaultHasher::new();
        key_a1.hash(&mut hasher1);
        let hash1 = hasher1.finish();

        let mut hasher2 = DefaultHasher::new();
        key_a2.hash(&mut hasher2);
        let hash2 = hasher2.finish();

        let mut hasher3 = DefaultHasher::new();
        button_left.hash(&mut hasher3);
        let hash3 = hasher3.finish();

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_shared_input_state_key_press_release() {
        let state = SharedInputState::new();

        state.update_key_press(Key::KeyA);
        {
            let keys_pressed = state.keys_pressed.lock().unwrap();
            assert!(keys_pressed.contains(&InputItem::Key(Key::KeyA)));
        }

        state.update_key_release(Key::KeyA);
        {
            let keys_pressed = state.keys_pressed.lock().unwrap();
            assert!(!keys_pressed.contains(&InputItem::Key(Key::KeyA)));
        }
    }

    #[test]
    fn test_shared_input_state_button_press_release() {
        let state = SharedInputState::new();

        state.update_button_press(Button::Left);
        {
            let keys_pressed = state.keys_pressed.lock().unwrap();
            assert!(keys_pressed.contains(&InputItem::Button(Button::Left)));
        }

        state.update_button_release(Button::Left);
        {
            let keys_pressed = state.keys_pressed.lock().unwrap();
            assert!(!keys_pressed.contains(&InputItem::Button(Button::Left)));
        }
    }

    #[test]
    fn test_shared_input_state_mouse_position() {
        let state = SharedInputState::new();

        state.update_mouse_position(100, 200);
        {
            let pos = state.last_mouse_position.lock().unwrap();
            assert_eq!(*pos, Some(MousePosition { x: 100, y: 200 }));
        }
    }

    #[test]
    fn test_shared_input_state_last_click_time() {
        let state = SharedInputState::new();

        state.update_button_press(Button::Left);
        {
            let last_click_time = state.last_click_time.lock().unwrap();
            assert!(last_click_time.is_some());
        }
    }

    #[test]
    fn test_shared_input_state_execute_callbacks() {
        let state = SharedInputState::new();

        let callback_called = Arc::new(Mutex::new(false));
        let callback_called_clone = Arc::clone(&callback_called);

        let callback = Box::new(move |_event: Event| {
            let mut called = callback_called_clone.lock().unwrap();
            *called = true;
        });

        {
            let mut callbacks = state.callbacks.lock().unwrap();
            callbacks.push(callback);
        }

        let event = Event {
            event_type: EventType::KeyPress(Key::KeyA),
            time: SystemTime::now(),
            name: None,
        };

        state.execute_callbacks(&event);

        assert!(*callback_called.lock().unwrap());
    }

    #[test]
    fn test_input_listener_is_key_pressed() {
        let listener = InputListener::new();

        listener.state.update_key_press(Key::KeyA);
        assert!(listener.is_key_pressed(Key::KeyA));

        listener.state.update_key_release(Key::KeyA);
        assert!(!listener.is_key_pressed(Key::KeyA));
    }

    #[test]
    fn test_input_listener_is_button_pressed() {
        let listener = InputListener::new();

        listener.state.update_button_press(Button::Left);
        assert!(listener.is_button_pressed(Button::Left));

        listener.state.update_button_release(Button::Left);
        assert!(!listener.is_button_pressed(Button::Left));
    }

    #[test]
    fn test_input_listener_get_keys_pressed() {
        let listener = InputListener::new();

        listener.state.update_key_press(Key::KeyA);
        listener.state.update_key_press(Key::KeyB);

        let keys_pressed = listener.get_keys_pressed();
        let expected_keys: BTreeSet<_> = vec![InputItem::Key(Key::KeyA), InputItem::Key(Key::KeyB)]
            .into_iter()
            .collect();

        assert_eq!(keys_pressed, expected_keys);
    }

    #[test]
    fn test_input_listener_get_last_mouse_position() {
        let listener = InputListener::new();

        listener.state.update_mouse_position(150, 250);

        let position = listener.get_last_mouse_position();
        assert_eq!(position, Some(MousePosition { x: 150, y: 250 }));
    }

    #[test]
    fn test_input_listener_get_last_click_time() {
        let listener = InputListener::new();

        listener.state.update_button_press(Button::Left);

        let last_click_time = listener.get_last_click_time();
        assert!(last_click_time.is_some());
    }

    #[test]
    fn test_input_listener_add_callback() {
        let listener = InputListener::new();

        let callback_called = Arc::new(Mutex::new(false));
        let callback_called_clone = Arc::clone(&callback_called);

        listener.add_callback(Box::new(move |_event: Event| {
            let mut called = callback_called_clone.lock().unwrap();
            *called = true;
        }));

        let event = Event {
            event_type: EventType::KeyPress(Key::KeyA),
            time: SystemTime::now(),
            name: None,
        };

        // Directly execute callbacks for testing
        listener.state.execute_callbacks(&event);

        assert!(*callback_called.lock().unwrap());
    }

    #[test]
    fn test_mouse_position_equality() {
        let pos1 = MousePosition { x: 100, y: 200 };
        let pos2 = MousePosition { x: 100, y: 200 };
        let pos3 = MousePosition { x: 150, y: 250 };

        assert_eq!(pos1, pos2);
        assert_ne!(pos1, pos3);
    }

    #[test]
    fn test_last_click_time_update() {
        let state = SharedInputState::new();

        state.update_button_press(Button::Left);
        let time1 = state.last_click_time.lock().unwrap().unwrap();
        std::thread::sleep(Duration::from_millis(10));
        state.update_button_press(Button::Right);
        let time2 = state.last_click_time.lock().unwrap().unwrap();

        assert!(time2 > time1);
    }

    #[test]
    fn test_input_action_display_order() {
        let action = InputAction::from_str("b+a+ctrl").unwrap();
        assert_eq!(format!("{}", action), "ControlLeft+KeyA+KeyB");
    }
}
