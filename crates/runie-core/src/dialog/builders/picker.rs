//! Theme picker and file picker dialog builders.

use super::{ItemAction, Panel, PanelStack};
use crate::Event;

/// Build a theme picker panel that applies the theme on Enter without
/// closing the dialog (live preview).
pub fn theme_picker(themes: Vec<(String, Event)>) -> PanelStack {
    let mut panel = Panel::new("theme", " Choose Theme ").keep_open();
    panel = panel.header("available themes — press Enter to preview");
    for (name, evt) in themes {
        panel = panel.item(name, ItemAction::Emit(evt));
    }
    PanelStack::new(panel)
}

/// Build an @-file picker panel.
pub fn file_picker(entries: Vec<(String, bool, Event)>) -> PanelStack {
    // (name, is_dir, event_to_emit)
    let is_empty = entries.is_empty();
    let mut panel = Panel::new("at-files", " Files ").with_filter();
    if is_empty {
        panel = panel.header("No files found");
    } else {
        panel = panel.header(format!("{} files", entries.len()));
    }
    for (name, is_dir, evt) in entries {
        let label = if is_dir { format!("{}/", name) } else { name };
        panel = panel.item(label, ItemAction::Emit(evt));
    }
    // `selected` is an index in the navigable-item list, not the raw item
    // vector. The first file is therefore navigable index zero even though a
    // non-selectable header precedes it in `items`.
    if !is_empty {
        panel.selected = 0;
    }
    PanelStack::new(panel)
}
