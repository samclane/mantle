use serde::{Deserialize, Serialize};

use super::input_item::InputItem;
use std::collections::BTreeSet;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::ops::{Deref, DerefMut};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]
pub struct InputAction(BTreeSet<InputItem>);

impl InputAction {
    pub fn new(action: BTreeSet<InputItem>) -> Self {
        InputAction(action)
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

impl Display for InputAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let mut items: Vec<String> = self.iter().map(|item| item.to_string()).collect();
        items.sort();
        write!(f, "{}", items.join("+"))
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
