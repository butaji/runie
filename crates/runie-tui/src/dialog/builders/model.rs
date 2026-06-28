//! Scoped models dialog builder.

use super::{ItemAction, Panel, PanelStack};
use runie_core::Event;

/// Build a scoped models panel with provider-grouped toggle items.
pub fn scoped_models(
    models: Vec<(String, String, bool)>, // (provider, name, enabled)
) -> PanelStack {
    let mut panel = Panel::new("scoped", " Scoped Models ").keep_open();
    let mut last_provider = String::new();
    for (provider, name, enabled) in models {
        if provider != last_provider {
            if !last_provider.is_empty() {
                panel = panel.separator();
            }
            panel = panel.header(provider.clone());
            last_provider = provider.clone();
        }
        // Emit a toggle event for each model — the state will mutate.
        let evt = Event::ScopedModelToggle {
            provider: provider.clone(),
            name: name.clone(),
        };
        panel = panel.toggle(name, enabled, ItemAction::Emit(evt));
    }
    PanelStack::new(panel)
}
