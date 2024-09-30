use eframe::egui;
use log::error;
use rdev::{listen, Button, Event, EventType, Key};
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread::{spawn, JoinHandle};
use std::time::Instant;

pub type BackgroundCallback = Box<dyn Fn(Event) + Send>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputItem {
    Key(Key),
    Button(Button),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InputAction(pub BTreeSet<InputItem>);

impl FromStr for InputItem {
    fn from_str(s: &str) -> Result<InputItem, ()> {
        match s.to_ascii_lowercase().as_str() {
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
            "a" => Ok(InputItem::Key(Key::KeyA)),
            "b" => Ok(InputItem::Key(Key::KeyB)),
            "c" => Ok(InputItem::Key(Key::KeyC)),
            "d" => Ok(InputItem::Key(Key::KeyD)),
            "e" => Ok(InputItem::Key(Key::KeyE)),
            "f" => Ok(InputItem::Key(Key::KeyF)),
            "g" => Ok(InputItem::Key(Key::KeyG)),
            "h" => Ok(InputItem::Key(Key::KeyH)),
            "i" => Ok(InputItem::Key(Key::KeyI)),
            "j" => Ok(InputItem::Key(Key::KeyJ)),
            "k" => Ok(InputItem::Key(Key::KeyK)),
            "l" => Ok(InputItem::Key(Key::KeyL)),
            "m" => Ok(InputItem::Key(Key::KeyM)),
            "n" => Ok(InputItem::Key(Key::KeyN)),
            "o" => Ok(InputItem::Key(Key::KeyO)),
            "p" => Ok(InputItem::Key(Key::KeyP)),
            "q" => Ok(InputItem::Key(Key::KeyQ)),
            "r" => Ok(InputItem::Key(Key::KeyR)),
            "s" => Ok(InputItem::Key(Key::KeyS)),
            "t" => Ok(InputItem::Key(Key::KeyT)),
            "u" => Ok(InputItem::Key(Key::KeyU)),
            "v" => Ok(InputItem::Key(Key::KeyV)),
            "w" => Ok(InputItem::Key(Key::KeyW)),
            "x" => Ok(InputItem::Key(Key::KeyX)),
            "y" => Ok(InputItem::Key(Key::KeyY)),
            "z" => Ok(InputItem::Key(Key::KeyZ)),
            _ => Err(error!("Failed to parse InputAction from string: {}", s)),
        }
    }
    type Err = ();
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

impl Hash for InputItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            InputItem::Key(k) => k.hash(state),
            InputItem::Button(b) => b.hash(state),
        }
    }
}

impl Ord for InputItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (InputItem::Key(k1), InputItem::Key(k2)) => {
                format!("{:?}", k1).cmp(&format!("{:?}", k2))
            }
            (InputItem::Button(b1), InputItem::Button(b2)) => {
                format!("{:?}", b1).cmp(&format!("{:?}", b2))
            }
            (InputItem::Key(_), InputItem::Button(_)) => std::cmp::Ordering::Less,
            (InputItem::Button(_), InputItem::Key(_)) => std::cmp::Ordering::Greater,
        }
    }
}

impl PartialOrd for InputItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for InputItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            InputItem::Key(k) => write!(f, "{:?}", k),
            InputItem::Button(b) => write!(f, "{:?}", b),
        }
    }
}

impl FromStr for InputAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split('+');
        let mut items = BTreeSet::new();

        for part in parts {
            let part = part.trim();
            if let Ok(item) = InputItem::from_str(part) {
                items.insert(item);
            } else {
                return Err(format!("Failed to parse InputAction from string: {}", s));
            }
        }

        Ok(InputAction(items))
    }
}

impl Display for InputAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut items: Vec<String> = self.0.iter().map(|item| format!("{}", item)).collect();
        items.sort(); // Ensure consistent order
        write!(f, "{}", items.join("+"))
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

    if let Ok(rdev_key) = map_egui_key_to_rdev_key(key) {
        items.insert(InputItem::Key(rdev_key));
    } else {
        error!("Failed to map egui::Key to rdev::Key: {:?}", key);
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
        if let Ok(mut keys) = self.keys_pressed.lock() {
            keys.insert(input_key);
        } else {
            error!("Failed to lock keys_pressed mutex");
        }
    }

    fn update_input_key_release(&self, input_key: InputItem) {
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
        self.update_input_key_press(InputItem::Button(button));

        if let Ok(mut time) = self.last_click_time.lock() {
            *time = Some(Instant::now());
        } else {
            error!("Failed to lock last_click_time mutex");
        }
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
        if let Ok(callbacks) = self.callbacks.lock() {
            for callback in callbacks.iter() {
                callback(event.clone());
            }
        } else {
            error!("Failed to lock callbacks mutex");
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

    pub fn is_input_key_pressed(&self, input_key: InputItem) -> bool {
        match self.state.keys_pressed.lock() {
            Ok(guard) => guard.contains(&input_key),
            Err(e) => {
                error!("Failed to lock keys_pressed mutex: {}", e);
                false
            }
        }
    }

    pub fn is_key_pressed(&self, key: Key) -> bool {
        self.is_input_key_pressed(InputItem::Key(key))
    }

    pub fn is_button_pressed(&self, button: Button) -> bool {
        self.is_input_key_pressed(InputItem::Button(button))
    }

    pub fn get_keys_pressed(&self) -> BTreeSet<InputItem> {
        match self.state.keys_pressed.lock() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to lock keys_pressed mutex: {}", e);
                BTreeSet::new()
            }
        }
    }

    pub fn is_input_action_pressed(&self, input_action: &InputAction) -> bool {
        match self.state.keys_pressed.lock() {
            Ok(guard) => input_action.0.is_subset(&guard),
            Err(e) => {
                error!("Failed to lock keys_pressed mutex: {}", e);
                false
            }
        }
    }

    pub fn get_items_pressed(&self) -> BTreeSet<InputItem> {
        match self.state.keys_pressed.lock() {
            Ok(guard) => guard.clone(),
            Err(e) => {
                error!("Failed to lock keys_pressed mutex: {}", e);
                BTreeSet::new()
            }
        }
    }

    pub fn add_callback(&self, callback: BackgroundCallback) {
        if let Ok(mut callbacks) = self.state.callbacks.lock() {
            callbacks.push(callback);
        } else {
            error!("Failed to lock callbacks mutex");
        }
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
    use std::time::SystemTime;

    use super::*;
    use rdev::{Button, Event, EventType, Key};

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

        // Simulate key press
        state.update_key_press(Key::KeyA);
        {
            let keys_pressed = state.keys_pressed.lock().unwrap();
            assert!(keys_pressed.contains(&InputItem::Key(Key::KeyA)));
        }

        // Simulate key release
        state.update_key_release(Key::KeyA);
        {
            let keys_pressed = state.keys_pressed.lock().unwrap();
            assert!(!keys_pressed.contains(&InputItem::Key(Key::KeyA)));
        }
    }

    #[test]
    fn test_shared_input_state_button_press_release() {
        let state = SharedInputState::new();

        // Simulate button press
        state.update_button_press(Button::Left);
        {
            let keys_pressed = state.keys_pressed.lock().unwrap();
            assert!(keys_pressed.contains(&InputItem::Button(Button::Left)));
        }

        // Simulate button release
        state.update_button_release(Button::Left);
        {
            let keys_pressed = state.keys_pressed.lock().unwrap();
            assert!(!keys_pressed.contains(&InputItem::Button(Button::Left)));
        }
    }

    #[test]
    fn test_shared_input_state_mouse_position() {
        let state = SharedInputState::new();

        // Simulate mouse move
        state.update_mouse_position(100, 200);
        {
            let pos = state.last_mouse_position.lock().unwrap();
            assert_eq!(*pos, Some(MousePosition { x: 100, y: 200 }));
        }
    }

    #[test]
    fn test_shared_input_state_last_click_time() {
        let state = SharedInputState::new();

        // Simulate button press
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

        // Simulate key press
        listener.state.update_key_press(Key::KeyA);
        assert!(listener.is_key_pressed(Key::KeyA));

        // Simulate key release
        listener.state.update_key_release(Key::KeyA);
        assert!(!listener.is_key_pressed(Key::KeyA));
    }

    #[test]
    fn test_input_listener_is_button_pressed() {
        let listener = InputListener::new();

        // Simulate button press
        listener.state.update_button_press(Button::Left);
        assert!(listener.is_button_pressed(Button::Left));

        // Simulate button release
        listener.state.update_button_release(Button::Left);
        assert!(!listener.is_button_pressed(Button::Left));
    }

    #[test]
    fn test_input_listener_get_keys_pressed() {
        let listener = InputListener::new();

        // Simulate key presses
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

        // Simulate mouse move
        listener.state.update_mouse_position(150, 250);

        let position = listener.get_last_mouse_position();
        assert_eq!(position, Some(MousePosition { x: 150, y: 250 }));
    }

    #[test]
    fn test_input_listener_get_last_click_time() {
        let listener = InputListener::new();

        // Simulate button press
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
}
