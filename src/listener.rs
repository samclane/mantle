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

#[derive(Clone, Copy, Debug)]
pub enum InputAction {
    Key(Key),
    Button(Button),
    Unknown,
}

impl FromStr for InputAction {
    fn from_str(s: &str) -> Result<InputAction, ()> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "a" => InputAction::Key(Key::KeyA),
            "b" => InputAction::Key(Key::KeyB),
            "c" => InputAction::Key(Key::KeyC),
            "d" => InputAction::Key(Key::KeyD),
            "e" => InputAction::Key(Key::KeyE),
            "f" => InputAction::Key(Key::KeyF),
            "g" => InputAction::Key(Key::KeyG),
            "h" => InputAction::Key(Key::KeyH),
            "i" => InputAction::Key(Key::KeyI),
            "j" => InputAction::Key(Key::KeyJ),
            "k" => InputAction::Key(Key::KeyK),
            "l" => InputAction::Key(Key::KeyL),
            "m" => InputAction::Key(Key::KeyM),
            "n" => InputAction::Key(Key::KeyN),
            "o" => InputAction::Key(Key::KeyO),
            "p" => InputAction::Key(Key::KeyP),
            "q" => InputAction::Key(Key::KeyQ),
            "r" => InputAction::Key(Key::KeyR),
            "s" => InputAction::Key(Key::KeyS),
            "t" => InputAction::Key(Key::KeyT),
            "u" => InputAction::Key(Key::KeyU),
            "v" => InputAction::Key(Key::KeyV),
            "w" => InputAction::Key(Key::KeyW),
            "x" => InputAction::Key(Key::KeyX),
            "y" => InputAction::Key(Key::KeyY),
            "z" => InputAction::Key(Key::KeyZ),
            _ => return Err(error!("Failed to parse InputAction from string: {}", s)),
        })
    }

    type Err = ();
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
            InputAction::Unknown => "Unknown".hash(state),
        }
    }
}

impl Ord for InputAction {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (InputAction::Key(k1), InputAction::Key(k2)) => {
                format!("{:?}", k1).cmp(&format!("{:?}", k2))
            }
            (InputAction::Button(b1), InputAction::Button(b2)) => {
                format!("{:?}", b1).cmp(&format!("{:?}", b2))
            }
            (InputAction::Key(_), InputAction::Button(_)) => std::cmp::Ordering::Less,
            (InputAction::Button(_), InputAction::Key(_)) => std::cmp::Ordering::Greater,
            (InputAction::Unknown, InputAction::Unknown) => std::cmp::Ordering::Equal,
            (InputAction::Unknown, _) => std::cmp::Ordering::Less,
            (_, InputAction::Unknown) => std::cmp::Ordering::Greater,
        }
    }
}

impl PartialOrd for InputAction {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for InputAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            InputAction::Key(k) => write!(f, "{:?}", k),
            InputAction::Button(b) => write!(f, "{:?}", b),
            InputAction::Unknown => write!(f, "Unknown"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MousePosition {
    pub x: i32,
    pub y: i32,
}

pub struct SharedInputState {
    last_mouse_position: Mutex<Option<MousePosition>>,
    last_click_time: Mutex<Option<Instant>>,
    keys_pressed: Mutex<BTreeSet<InputAction>>,
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
        let key_a = InputAction::Key(Key::KeyA);
        let key_a2 = InputAction::Key(Key::KeyA);
        let key_b = InputAction::Key(Key::KeyB);
        let button_left = InputAction::Button(Button::Left);
        let button_right = InputAction::Button(Button::Right);

        assert_eq!(key_a, key_a2);
        assert_ne!(key_a, key_b);
        assert_ne!(key_a, button_left);
        assert_ne!(button_left, button_right);
    }

    #[test]
    fn test_input_action_ordering() {
        let mut actions = vec![
            InputAction::Key(Key::KeyB),
            InputAction::Button(Button::Left),
            InputAction::Key(Key::KeyA),
            InputAction::Button(Button::Right),
        ];

        actions.sort();

        assert_eq!(
            actions,
            vec![
                InputAction::Key(Key::KeyA),
                InputAction::Key(Key::KeyB),
                InputAction::Button(Button::Left),
                InputAction::Button(Button::Right),
            ]
        );
    }

    #[test]
    fn test_input_action_display() {
        let key = InputAction::Key(Key::KeyA);
        let button = InputAction::Button(Button::Left);

        assert_eq!(format!("{}", key), "KeyA");
        assert_eq!(format!("{}", button), "Left");
    }

    #[test]
    fn test_input_action_hash() {
        use std::collections::hash_map::DefaultHasher;

        let key_a1 = InputAction::Key(Key::KeyA);
        let key_a2 = InputAction::Key(Key::KeyA);
        let button_left = InputAction::Button(Button::Left);

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
            assert!(keys_pressed.contains(&InputAction::Key(Key::KeyA)));
        }

        // Simulate key release
        state.update_key_release(Key::KeyA);
        {
            let keys_pressed = state.keys_pressed.lock().unwrap();
            assert!(!keys_pressed.contains(&InputAction::Key(Key::KeyA)));
        }
    }

    #[test]
    fn test_shared_input_state_button_press_release() {
        let state = SharedInputState::new();

        // Simulate button press
        state.update_button_press(Button::Left);
        {
            let keys_pressed = state.keys_pressed.lock().unwrap();
            assert!(keys_pressed.contains(&InputAction::Button(Button::Left)));
        }

        // Simulate button release
        state.update_button_release(Button::Left);
        {
            let keys_pressed = state.keys_pressed.lock().unwrap();
            assert!(!keys_pressed.contains(&InputAction::Button(Button::Left)));
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
        let expected_keys: BTreeSet<_> =
            vec![InputAction::Key(Key::KeyA), InputAction::Key(Key::KeyB)]
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
