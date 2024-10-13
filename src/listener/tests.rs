#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeSet,
        hash::{Hash, Hasher},
        sync::{Arc, Mutex},
        time::{Duration, SystemTime},
    };

    use eframe::egui;
    use rdev::{Button, Event, EventType, Key};

    use crate::listener::{
        input_action::InputAction,
        input_item::InputItem,
        input_listener::InputListener,
        key_mapping::{from_egui, map_egui_key_to_rdev_key},
        shared_input_state::{MousePosition, SharedInputState},
    };

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
        assert_eq!(input_action, expected_items);
    }

    #[test]
    fn test_is_input_action_pressed() {
        let listener = InputListener::new();
        listener.state.update_key_press(Key::ControlLeft);
        listener.state.update_key_press(Key::KeyA);

        let action = InputAction::from_iter(vec![
            InputItem::Key(Key::ControlLeft),
            InputItem::Key(Key::KeyA),
        ]);
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
}
