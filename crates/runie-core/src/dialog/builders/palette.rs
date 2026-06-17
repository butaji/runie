//! Command palette and model selector builders.

use super::{ItemAction, Panel, PanelStack};
use crate::event::ModelConfigEvent;
use crate::Event;

/// Build a command palette panel from structured command rows.
///
/// Mirrors [`settings`](fn@settings): the caller passes typed rows and the
/// builder turns them into panel items, avoiding stringly-typed labels.
pub fn command_palette(items: Vec<crate::commands::CommandRow>) -> PanelStack {
    // keep_open_on_activate is required for Android-like back-stack
    // navigation: selecting a command from the palette pushes the
    // palette onto the global back stack, so Esc on the sub-dialog
    // returns to the palette instead of closing the whole bar.
    let mut panel = Panel::new("palette", " Commands ")
        .with_filter()
        .keep_open();
    let mut last_category = String::new();
    for row in items {
        if row.category != last_category {
            if !last_category.is_empty() {
                panel = panel.separator();
            }
            panel = panel.header(row.category.clone());
            last_category = row.category;
        }
        panel = panel.command(row.name, row.desc, ItemAction::Emit(row.event));
    }
    PanelStack::new(panel)
}

/// Build a model selector panel with provider-grouped items.
///
/// `groups` is a list of `(provider_name, [(model_name, event_to_emit_on_select)])`.
pub fn model_selector(
    recent: Vec<String>,
    groups: Vec<(String, Vec<(String, Event)>)>,
    current: &str,
) -> PanelStack {
    let mut panel = Panel::new("model", " Select Model ").with_filter();

    if !recent.is_empty() {
        panel = panel.header("Recent");
        for model in recent {
            let evt = ModelConfigEvent::SwitchModel {
                provider: model.split('/').next().unwrap_or("").into(),
                model: model.clone(),
            };
            let label = if model == current {
                format!("★ {}", model)
            } else {
                model
            };
            panel = panel.item(label, ItemAction::Emit(evt));
        }
        panel = panel.separator();
    }

    for (provider, models) in groups {
        panel = panel.header(provider);
        for (name, evt) in models {
            let label = if name == current {
                format!("★ {}", name)
            } else {
                name
            };
            panel = panel.item(label, ItemAction::Emit(evt));
        }
    }
    PanelStack::new(panel)
}
