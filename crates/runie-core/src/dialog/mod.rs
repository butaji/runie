//! Declarative dialog DSL with stackable panels for nested navigation.
//!
//! A `Dialog` holds a stack of `Panel`s. Only the top panel is visible.
//! Push a panel to drill down, pop to go back. Each panel contains items
//! that can be navigated with ↑/↓ and activated with Enter.

mod panel;
mod stack;

#[cfg(test)]
mod tests;

pub use panel::{Panel, PanelItem, ItemAction};
pub use stack::{PanelStack, PanelId};
