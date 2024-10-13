use rdev::{Button, Key};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
