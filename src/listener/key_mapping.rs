use std::collections::BTreeSet;
use std::fmt::{Display, Formatter, Result as FmtResult};

use super::input_action::InputAction;
use super::input_item::InputItem;
use eframe::egui;
use log::error;
use rdev::Key;

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
        // Navigation / command keys
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

        // Punctuation
        egui::Key::Comma => Ok(Key::Comma),
        egui::Key::Backslash => Ok(Key::BackSlash),
        egui::Key::Slash => Ok(Key::Slash),
        egui::Key::OpenBracket => Ok(Key::LeftBracket),
        egui::Key::CloseBracket => Ok(Key::RightBracket),
        egui::Key::Backtick => Ok(Key::BackQuote),
        egui::Key::Minus => Ok(Key::Minus),
        egui::Key::Period => Ok(Key::Dot),
        egui::Key::Equals => Ok(Key::Equal),
        egui::Key::Semicolon => Ok(Key::SemiColon),
        egui::Key::Quote => Ok(Key::Quote),

        // Digits
        egui::Key::Num0 => Ok(Key::Num0),
        egui::Key::Num1 => Ok(Key::Num1),
        egui::Key::Num2 => Ok(Key::Num2),
        egui::Key::Num3 => Ok(Key::Num3),
        egui::Key::Num4 => Ok(Key::Num4),
        egui::Key::Num5 => Ok(Key::Num5),
        egui::Key::Num6 => Ok(Key::Num6),
        egui::Key::Num7 => Ok(Key::Num7),
        egui::Key::Num8 => Ok(Key::Num8),
        egui::Key::Num9 => Ok(Key::Num9),

        // Letters
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

        // Function keys (F1-F12 only; rdev has no F13+ variants)
        egui::Key::F1 => Ok(Key::F1),
        egui::Key::F2 => Ok(Key::F2),
        egui::Key::F3 => Ok(Key::F3),
        egui::Key::F4 => Ok(Key::F4),
        egui::Key::F5 => Ok(Key::F5),
        egui::Key::F6 => Ok(Key::F6),
        egui::Key::F7 => Ok(Key::F7),
        egui::Key::F8 => Ok(Key::F8),
        egui::Key::F9 => Ok(Key::F9),
        egui::Key::F10 => Ok(Key::F10),
        egui::Key::F11 => Ok(Key::F11),
        egui::Key::F12 => Ok(Key::F12),

        // No rdev equivalent: Copy, Cut, Paste, Colon, Pipe, Questionmark, Plus, F13-F35
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

    InputAction::from(items)
}
