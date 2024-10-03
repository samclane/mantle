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

    InputAction::from(items)
}
