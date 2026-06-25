//! Core TUI integration tests moved from runie-core.

// Re-export types used by child test modules so they can `use super::*;`
pub use super::{AppState, ChatMessage, Element, Event, Part, PermissionRequestState, Role,
    ScopedModel, Snapshot};

#[cfg(test)]
mod action_text;
#[cfg(test)]
mod at_file_picker;
#[cfg(test)]
mod autoscroll_bug;
#[cfg(test)]
mod autoscroll_overflow;
#[cfg(test)]
mod collapse;
#[cfg(test)]
mod collapse_new_items;
#[cfg(test)]
mod dialog_theme_switch;
#[cfg(test)]
mod element_dsl;
#[cfg(test)]
mod element_order;
#[cfg(test)]
mod element_sorting;
#[cfg(test)]
mod element_spacing;
#[cfg(test)]
mod input;
#[cfg(test)]
mod line_scroll;
#[cfg(test)]
mod mouse_events;
#[cfg(test)]
mod no_ghost_agent;
#[cfg(test)]
mod palette;
#[cfg(test)]
mod paragraph_scroll;
#[cfg(test)]
mod scrollbar;
#[cfg(test)]
mod semantic_order;
#[cfg(test)]
mod settings_dialog;
#[cfg(test)]
mod status_timer;
#[cfg(test)]
mod tab_complete;
#[cfg(test)]
mod tab_file_picker_filter;
#[cfg(test)]
mod thinking;
#[cfg(test)]
mod toggle_all;
#[cfg(test)]
mod toggle_stress;
#[cfg(test)]
pub(crate) mod visible_helper;
