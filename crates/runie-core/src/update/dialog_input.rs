use crate::actors::TurnMsg;
use crate::model::{AppState, InputReceiver};

use super::dialog;

impl AppState {
    pub(super) fn try_handle_dialog_event_input(&mut self, event: &crate::Event) -> bool {
        // Use open_dialog_mut() for both the read check and the write.
        // This avoids the immutable+mutable accessor borrow conflict.
        let dialog = self.open_dialog_mut();

        // No dialog open: let event pass through to normal input handling
        if dialog.is_none() {
            return false;
        }

        let is_welcome = matches!(dialog.as_ref(), Some(crate::commands::DialogState::Welcome));
        if is_welcome {
            match event {
                crate::Event::Input(_) | crate::Event::Submit => {
                    *dialog = None;
                    self.view_mut().input_receiver = InputReceiver::ChatInput;
                    self.view_mut().dirty = true;
                    return false; // also pass to input handler
                }
                _ => return false, // let other keys pass through to input
            }
        }
        let _ = dialog; // release mutable borrow before dialog::update_dialog
        match event {
            crate::Event::Input(_)
            | crate::Event::Submit
            | crate::Event::Backspace
            | crate::Event::HistoryPrev
            | crate::Event::HistoryNext
            | crate::Event::CursorLeft
            | crate::Event::CursorRight
            | crate::Event::Paste(_) => {
                dialog::update_dialog(self, event.clone());
                return true;
            }
            // intentionally ignored: other input events fall through
            _ => {}
        }
        false
    }

    pub(super) fn try_handle_vim_dialog_back_input(&mut self, event: &crate::Event) -> bool {
        if *event != crate::Event::Backspace || !self.view_mut().vim_nav_mode {
            return false;
        }
        self.handle_vim_dialog_back();
        true
    }

    pub(super) fn try_handle_vim_nav_event_input(&mut self, event: &crate::Event) -> bool {
        if !self.view_mut().vim_nav_mode {
            return false;
        }
        let Some(handled) = self.handle_vim_nav_event(event) else {
            return false;
        };
        !handled
    }

    pub(super) fn try_handle_dialog_event_dialog(&mut self, event: &crate::Event) -> bool {
        if self.open_dialog().is_none() {
            return false;
        }
        if self.login_flow().is_some() && matches!(event, crate::Event::ProvidersAdd) {
            return false;
        }
        dialog::update_dialog(self, event.clone());
        true
    }

    pub(crate) fn handle_vim_dialog_back(&mut self) {
        let view = self.view_mut();
        if view.input_receiver == InputReceiver::Dialog {
            view.input_receiver = InputReceiver::ChatInput;
            view.dirty = true;
            return;
        }
        if view.vim_nav_mode {
            view.vim_nav_mode = false;
            view.dirty = true;
            return;
        }
        if view.vim_nav_pending {
            view.vim_nav_pending = false;
            view.vim_nav_mode = true;
            self.view_mut().selected_post = self.current_bottom_post_index();
            return;
        }
        if self.agent_state().turn_active {
            self.abort_turn_for_vim_nav();
            return;
        }
        {
            let view = self.view_mut();
            view.vim_nav_mode = true;
        }
        self.view_mut().selected_post = self.current_bottom_post_index();
    }

    /// Abort turn when entering vim nav mode.
    fn abort_turn_for_vim_nav(&mut self) {
        let handles = self.actor_handles().cloned();
        if let Some(ref h) = handles {
            if tokio::runtime::Handle::try_current().is_ok() {
                let _ = h.turn.try_send(TurnMsg::AbortTurn);
            }
        } else {
            self.apply_turn_aborted();
        }
        self.view_mut().vim_nav_pending = true;
        self.view_mut().dirty = true;
    }

    pub(crate) fn current_bottom_post_index(&mut self) -> Option<usize> {
        let snap = self.snapshot();
        let view = self.view_mut();
        let bottom = crate::snapshot::compute_current_bottom_element(
            &snap.elements,
            &snap.line_counts,
            snap.total_lines,
            snap.scroll,
            view.last_visible_height,
        )?;
        snap.posts
            .iter()
            .find(|p| p.start <= bottom && bottom < p.end)
            .map(|p| p.index)
    }
}
