use rdev::{Button, Key};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::str::FromStr;

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

impl Serialize for InputItem {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            InputItem::Key(k) => serializer.serialize_str(&format!("{:?}", k)),
            InputItem::Button(b) => serializer.serialize_str(&format!("{:?}", b)),
        }
    }
}

impl<'de> Deserialize<'de> for InputItem {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        InputItem::from_str(&s).map_err(serde::de::Error::custom)
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
        let s_strip = s_lower
            .trim_start_matches("key")
            .trim_start_matches("button")
            .to_string();
        match s_strip.as_str() {
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
