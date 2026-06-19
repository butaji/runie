//! Settings dialog builder.
//!
//! Accepts pre-built panel items grouped by category and renders them as a
//! single scrollable panel with category headers.

use super::{Panel, PanelStack};

/// Build a settings dialog panel with category headers.
pub fn settings(categories: Vec<(String, Vec<super::PanelItem>)>) -> PanelStack {
    let mut panel = Panel::new("settings", " Settings ");
    for (i, (cat_name, rows)) in categories.into_iter().enumerate() {
        if i > 0 {
            panel = panel.separator();
        }
        panel = panel.header(cat_name);
        for item in rows {
            panel.items.push(item);
        }
    }
    PanelStack::new(panel)
}
