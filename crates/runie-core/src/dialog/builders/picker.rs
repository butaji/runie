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
    let mut panel = Panel::new("at-files", " Files ").with_filter();
    if entries.is_empty() {
        panel = panel.header("No files found");
    } else {
        panel = panel.header(format!("{} files", entries.len()));
    }
    for (name, is_dir, evt) in entries {
        let label = if is_dir { format!("{}/", name) } else { name };
        panel = panel.item(label, ItemAction::Emit(evt));
    }
    PanelStack::new(panel)
}
