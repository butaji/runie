use crate::event::{DialogEvent, InputEvent};
use crate::model::AppState;

use super::dialog;

impl AppState {
    pub(super) fn try_handle_dialog_event_input(&mut self, event: &InputEvent) -> bool {
        if self.open_dialog.is_none() {
            return false;
        }
        // Welcome dialog closes on any printable input or Submit
        if matches!(
            self.open_dialog,
            Some(crate::commands::DialogState::Welcome)
        ) {
            match event {
                InputEvent::Input(_) | InputEvent::Submit => {
                    self.open_dialog = None;
                    self.mark_dirty();
                    return false; // also pass to input handler
                }
                _ => return false, // let other keys pass through to input
            }
        }
        match event {
            InputEvent::Input(_)
            | InputEvent::Submit
            | InputEvent::Backspace
            | InputEvent::HistoryPrev
            | InputEvent::HistoryNext
            | InputEvent::CursorLeft
            | InputEvent::CursorRight
            | InputEvent::Paste(_) => {
                dialog::update_dialog(self, event.clone());
                return true;
            }
            _ => {}
        }
        false
    }

    pub(super) fn try_handle_vim_dialog_back_input(&mut self, event: &InputEvent) -> bool {
        if *event != InputEvent::Backspace || !self.view.vim_nav_mode {
            return false;
        }
        self.handle_vim_dialog_back();
        true
    }

    pub(super) fn try_handle_vim_nav_event_input(&mut self, event: &InputEvent) -> bool {
        if !self.view.vim_nav_mode {
            return false;
        }
        let Some(handled) = self.handle_vim_nav_event(event) else {
            return false;
        };
        !handled
    }

    pub(super) fn try_handle_dialog_event_dialog(&mut self, event: &crate::Event) -> bool {
        if self.open_dialog.is_none() {
            return false;
        }
        if self.login_flow.is_some() && matches!(event, DialogEvent::ProvidersAdd) {
            return false;
        }
        dialog::update_dialog(self, event.clone());
        true
    }

    pub(crate) fn handle_vim_dialog_back(&mut self) {
        if self.view.vim_nav_mode {
            self.view.vim_nav_mode = false;
            self.mark_dirty();
            return;
        }
        if self.view.vim_nav_pending {
            self.view.vim_nav_pending = false;
            self.view.vim_nav_mode = true;
            self.mark_dirty();
            return;
        }
        if self.agent.turn_active {
            self.agent.turn_active = false;
            self.agent.inflight = 0;
            self.view.vim_nav_pending = true;
            self.mark_dirty();
            return;
        }
        self.view.vim_nav_mode = true;
        self.view.selected_post = self.current_bottom_post_index();
        self.mark_dirty();
    }

    pub(crate) fn current_bottom_post_index(&self) -> Option<usize> {
        let bottom = crate::snapshot::compute_current_bottom_element(
            &self.view.elements_cache,
            &self.view.line_counts,
            self.view.total_lines,
            self.view.scroll,
            self.view.last_visible_height,
        )?;
        self.view
            .posts
            .iter()
            .find(|p| p.start <= bottom && bottom < p.end)
            .map(|p| p.index)
    }

    #[allow(dead_code)]
    fn handle_vim_nav_event_input(&mut self, _event: &InputEvent) -> Option<bool> {
        None
    }
}
