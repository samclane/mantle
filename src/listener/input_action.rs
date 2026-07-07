use super::input_item::InputItem;
use std::collections::BTreeSet;

/// A chord of keys/buttons that must all be held together.
pub type InputAction = BTreeSet<InputItem>;
