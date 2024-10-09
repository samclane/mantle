use serde::{Deserialize, Serialize};

use super::input_item::InputItem;
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct InputAction(BTreeSet<InputItem>);

impl InputAction {
    pub fn new() -> Self {
        InputAction(BTreeSet::new())
    }
}

impl Deref for InputAction {
    type Target = BTreeSet<InputItem>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for InputAction {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<InputItem> for InputAction {
    fn from_iter<I: IntoIterator<Item = InputItem>>(iter: I) -> Self {
        InputAction(iter.into_iter().collect())
    }
}

impl IntoIterator for InputAction {
    type Item = InputItem;
    type IntoIter = std::collections::btree_set::IntoIter<InputItem>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl Extend<InputItem> for InputAction {
    fn extend<I: IntoIterator<Item = InputItem>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl From<BTreeSet<InputItem>> for InputAction {
    fn from(items: BTreeSet<InputItem>) -> Self {
        InputAction(items)
    }
}

impl From<&[InputItem]> for InputAction {
    fn from(items: &[InputItem]) -> Self {
        InputAction(items.iter().cloned().collect())
    }
}

impl FromStr for InputAction {
    type Err = InputActionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.split('+')
            .map(str::trim)
            .map(InputItem::from_str)
            .collect::<Result<BTreeSet<_>, _>>()
            .map(InputAction)
            .map_err(|e| InputActionParseError::InvalidItem(e.to_string()))
    }
}

impl Display for InputAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut items: Vec<String> = self.iter().map(|item| item.to_string()).collect();
        items.sort();
        write!(f, "{}", items.join("+"))
    }
}

impl Serialize for InputAction {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let s = self.to_string();
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for InputAction {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        InputAction::from_str(&s).map_err(serde::de::Error::custom)
    }
}

impl PartialEq<BTreeSet<InputItem>> for InputAction {
    fn eq(&self, other: &BTreeSet<InputItem>) -> bool {
        self.0 == *other
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
