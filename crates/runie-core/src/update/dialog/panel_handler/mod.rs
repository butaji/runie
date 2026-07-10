//! Panel stack navigation and item activation.
//!
//! ## Modules
//!
//! - `navigation` — Close and navigation event handlers
//! - `activation` — Selection and action handling
//! - `filter` — Text filtering in panels
//! - `settings` — Settings application (toggles, checkboxes, selects)

mod activation;
mod filter;
mod navigation;
mod settings;

#[cfg(test)]
mod tests;

use crate::commands::{DialogKind, DialogState};
use crate::dialog::{Panel, PanelStack};
use crate::model::AppState;
use crate::Event;

use super::form::FormAction;

/// Result of handling a single event in a panel stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanelUpdateResult {
    /// Event was consumed and the dialog should remain open.
    Consumed,
    /// Event closed the dialog.
    Closed,
    /// Event was ignored by the panel stack.
    Ignored,
}

/// Whether the root panel of the active dialog allows dismissal.
pub(crate) fn root_closable(state: &AppState) -> bool {
    state
        .open_dialog
        .as_ref()
        .and_then(|d| d.panel_stack())
        .and_then(|s| s.root())
        .map(|p| p.closable)
        .unwrap_or(true)
}

/// Update a panel stack in response to an event.
pub fn update_panel_stack(
    state: &mut AppState,
    event: Event,
    stack: &mut PanelStack,
) -> PanelUpdateResult {
    let is_form = stack.current().is_some_and(|p| p.is_form());
    if is_form {
        return update_form_panel(state, event, stack);
    }

    if navigation::handle_panel_close(state, &event, stack) {
        return PanelUpdateResult::Closed;
    }
    if navigation::handle_panel_navigation(state, &event, stack) {
        return PanelUpdateResult::Consumed;
    }
    if let Some(result) = activation::handle_panel_activation(state, &event, stack) {
        return match result {
            activation::ActivationResult::Consumed => PanelUpdateResult::Consumed,
            activation::ActivationResult::Closed => PanelUpdateResult::Closed,
        };
    }
    filter::handle_panel_filter(state, &event, stack);
    state.view_mut().dirty = true;
    PanelUpdateResult::Ignored
}

/// Update a form panel.
fn update_form_panel(
    state: &mut AppState,
    event: Event,
    stack: &mut PanelStack,
) -> PanelUpdateResult {
    let action = {
        let panel = stack.current_mut().expect("form panel");
        super::form::form_panel_action(state, panel, event)
    };

    if matches!(&action, FormAction::Back) {
        return if handle_back_action(state, stack) {
            PanelUpdateResult::Closed
        } else {
            PanelUpdateResult::Consumed
        };
    }

    let keep_open = matches!(&action, FormAction::KeepOpen);
    // Restore dialog if closed by Back action (handle_back_action closes it).
    // For other actions (Submit, SubmitCommand), apply_form_action handles closure.
    if (!keep_open || matches!(&action, FormAction::Back)) && state.open_dialog().is_none() {
        *state.open_dialog_mut() = Some(DialogState::Active {
            kind: DialogKind::Generic,
            panels: stack.clone(),
        });
    }
    super::form::apply_form_action(state, action);
    if keep_open {
        PanelUpdateResult::Consumed
    } else {
        PanelUpdateResult::Closed
    }
}

fn handle_back_action(state: &mut AppState, stack: &mut PanelStack) -> bool {
    if stack.len() > 1 {
        stack.pop();
        *state.open_dialog_mut() = Some(DialogState::Active {
            kind: DialogKind::Generic,
            panels: stack.clone(),
        });
        false
    } else {
        let root_closable = stack.root().map(|p| p.closable).unwrap_or(true);
        navigation::pop_dialog_or_close(state, root_closable)
    }
}

/// Toggle the currently selected checkbox (if any) and apply its side effect.
/// Returns `true` if a toggle item was selected.
pub(super) fn toggle_selected_checkbox(state: &mut AppState, panel: &mut Panel) -> bool {
    let Some(item) = panel.selected_item_mut() else {
        return false;
    };
    settings::toggle_checkbox_item(state, item)
}
