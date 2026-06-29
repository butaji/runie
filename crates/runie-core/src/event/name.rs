//! Canonical event name mapping for bindable variants.
//!
//! Uses strum's `IntoStaticStr` for name extraction and the generated
//! `EVENT_NAMES` table for bindable-variant lookup.

use super::Event;
use crate::event::EVENT_NAMES;

impl Event {
    /// Canonical string name for bindable variants (those in EVENT_NAMES).
    /// Returns `None` for parameterized variants that can't be bound.
    pub fn name(&self) -> Option<&'static str> {
        let self_name: &'static str = self.into();
        // Check if this variant is in the bindable set (zero-arg variants).
        EVENT_NAMES
            .iter()
            .any(|(n, _)| *n == self_name)
            .then_some(self_name)
    }

    /// Build an Event from its canonical name. Supports `Input:<char>` prefix.
    pub fn from_name(name: &str) -> Option<Event> {
        if let Some(rest) = name.strip_prefix("Input:") {
            let c = rest.chars().next()?;
            return Some(Event::Input(c));
        }
        EVENT_NAMES
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, ctor)| ctor())
    }
}
