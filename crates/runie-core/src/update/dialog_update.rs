//! Dialog update routing.

use crate::model::AppState;
use crate::Event;
use crate::update::FormAction;

pub(crate) fn update(state: &mut AppState, event: Event) {
    if matches!(event, Event::Abort) {
        state.open_dialog = None;
        state.mark_dirty();
        return;
    }
    if matches!(
        event,
        Event::SwitchTheme { .. }
            | Event::SwitchModel { .. }
            | Event::CycleModelNext
            | Event::CycleModelPrev
            | Event::CycleThinkingLevel
            | Event::SetThinkingLevel(_)
            | Event::ToggleReadOnly
            | Event::TrustProject
            | Event::UntrustProject
    ) {
        super::model_config_event(state, event);
        return;
    }
    if matches!(event, Event::Quit) {
        state.should_quit = true;
        return;
    }
    let Some(mut dialog) = state.open_dialog.take() else {
        return;
    };
    let stack = dialog.panel_stack_mut();
    let activated = state.update_panel_stack(event, stack);
    // If the panel stack activated an item, it may have closed or replaced
    // the dialog (e.g. opening a settings dialog). Otherwise restore the
    // original variant so CommandPalette/Settings/etc. identity is preserved.
    // Exception: if the handler already set `open_dialog` (e.g. login flow
    // rebuild on keep_open Emit), leave it alone.
    if !activated && state.open_dialog.is_none() {
        state.open_dialog = Some(dialog);
    }
    state.mark_dirty();
}

impl AppState {
    /// Update a panel stack in response to an event. Returns `true` if an item
    /// was activated (which may have closed or replaced the dialog).
    pub(super) fn update_panel_stack(&mut self, event: Event, stack: &mut crate::dialog::PanelStack) -> bool {
        use Event::*;

        if stack.current().is_some_and(|p| p.is_form()) {
            return self.update_form_panel(event, stack);
        }
        match event {
            SettingsClose | PaletteClose | ModelSelectorClose => {
                if stack.len() == 1 {
                    self.open_dialog = None;
                    self.mark_dirty();
                    return true;
                }
                stack.pop();
            }
            HistoryPrev | SettingsUp | PaletteUp | ModelSelectorUp => stack.select_up(),
            HistoryNext | SettingsDown | PaletteDown | ModelSelectorDown => stack.select_down(),
            CursorLeft | SettingsLeft => {
                stack.pop();
            }
            Submit | SettingsSelect | PaletteSelect | ModelSelectorSelect => {
                // Always return after activation: the handler may have
                // replaced `open_dialog` (e.g. login flow rebuild), and
                // we must not overwrite it with the pre-activation stack.
                return self.try_activate_panel(stack);
            }
            PaletteFilter(c) | ModelSelectorFilter(c) | Input(c) => stack.push_filter(c),
            PaletteBackspace | ModelSelectorBackspace | Backspace => stack.pop_filter(),
            _ => {}
        }
        // The caller (`update_dialog`) persists the dialog by restoring the
        // original `DialogState` variant with the modified stack. We must
        // not overwrite `open_dialog` here, or the original variant
        // (e.g. CommandPalette, Settings) is lost. Handlers that need to
        // replace the dialog (e.g. login flow on keep_open Emit) set
        // `open_dialog` directly and return early via the Submit arm.
        self.mark_dirty();
        false
    }

    fn update_form_panel(&mut self, event: Event, stack: &mut crate::dialog::PanelStack) -> bool {
        let action = {
            let panel = stack.current_mut().expect("form panel");
            Self::form_panel_action(panel, event)
        };

        // Stack navigation: pop if the stack is deeper than the root,
        // otherwise close the entire dialog.
        if matches!(&action, FormAction::Back) {
            if stack.len() > 1 {
                stack.pop();
                self.open_dialog = Some(crate::commands::DialogState::PanelStack(stack.clone()));
                return false; // keep open with popped stack
            } else {
                self.open_dialog = None;
                self.mark_dirty();
                return true; // closed at root
            }
        }

        let keep_open = matches!(&action, FormAction::KeepOpen);
        if keep_open {
            self.open_dialog = Some(crate::commands::DialogState::PanelStack(stack.clone()));
        }
        self.apply_form_action(action);
        !keep_open
    }

    fn try_activate_panel(&mut self, stack: &mut crate::dialog::PanelStack) -> bool {
        if let Some(action) = stack.activate() {
            if self.handle_panel_action(action, stack) {
                return true;
            }
        }
        false
    }

}
