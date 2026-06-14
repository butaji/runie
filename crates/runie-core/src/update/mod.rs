use crate::model::AppState;
use crate::Event;

// Re-export for backward compatibility
pub use crate::tool_markers::has_tool_markers as content_has_tool_markers;
pub use crate::tool_markers::strip_tool_markers;

mod agent;
mod at_refs;
mod bash;
mod control;
mod dialog;
pub(crate) mod dialog_form;
mod dialog_panel;
mod dialog_toggle;
mod dispatch;
mod edit;
mod edit_approval;
mod form;
pub use form::FormAction;
mod input_dispatch;
mod input_history;
mod input_nav;
mod input_scroll;
mod input_text;
mod input_text_support;
mod line_nav;
mod login_flow;
mod model_config;
mod model_selector;
mod path_complete;
mod queue;
pub mod scoped_models;
mod scroll;
mod session;
pub mod settings_dialog;
mod state_helpers;
mod system_actions;
pub mod tab_complete;

pub(crate) fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

impl AppState {
    /// Main event dispatcher - delegates to specialized handlers based on event type.
    pub fn update(&mut self, event: Event) {
        if is_login_flow_event(&event) {
            login_flow::login_flow_event(self, event);
            return;
        }

        if is_providers_event(&event) {
            login_flow::providers_event(self, event);
            return;
        }

        if self.try_handle_dialog_event(&event) {
            return;
        }

        if self.try_handle_vim_dialog_back(&event) {
            return;
        }

        if self.try_handle_vim_nav_event(&event) {
            return;
        }

        dispatch::dispatch_event(self, event);
    }

    fn try_handle_dialog_event(&mut self, event: &Event) -> bool {
        if self.open_dialog.is_none() {
            return false;
        }
        if self.login_flow.is_some() && *event == Event::DialogBack {
            login_flow::login_flow_cancel(self);
            return true;
        }
        dialog::update_dialog(self, event.clone());
        true
    }

    fn try_handle_vim_dialog_back(&mut self, event: &Event) -> bool {
        if *event != Event::DialogBack || !self.config.vim_mode {
            return false;
        }
        self.handle_vim_dialog_back();
        true
    }

    fn try_handle_vim_nav_event(&mut self, event: &Event) -> bool {
        if !self.vim_nav_mode {
            return false;
        }
        let Some(handled) = self.handle_vim_nav_event(event) else {
            return false;
        };
        !handled
    }

    fn handle_vim_dialog_back(&mut self) {
        if self.vim_nav_mode {
            self.vim_nav_mode = false;
            self.mark_dirty();
            return;
        }
        if self.vim_nav_pending {
            self.vim_nav_pending = false;
            self.vim_nav_mode = true;
            self.mark_dirty();
            return;
        }
        if self.agent.turn_active {
            self.agent.turn_active = false;
            self.agent.inflight = 0;
            self.vim_nav_pending = true;
            self.mark_dirty();
            return;
        }
        self.vim_nav_mode = true;
        self.view.selected_post = self.current_bottom_post_index();
        self.mark_dirty();
    }

    fn current_bottom_post_index(&self) -> Option<usize> {
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
}

fn is_login_flow_event(event: &Event) -> bool {
    matches!(
        event,
        Event::LoginFlowStart
            | Event::LoginFlowSelectProvider { .. }
            | Event::LoginFlowSubmitKey { .. }
            | Event::LoginFlowValidationDone { .. }
            | Event::LoginFlowValidationFailed { .. }
            | Event::LoginFlowModelsFetched { .. }
            | Event::LoginFlowToggleModel { .. }
            | Event::LoginFlowSave
            | Event::LoginFlowCancel
    )
}

fn is_providers_event(event: &Event) -> bool {
    matches!(
        event,
        Event::ProvidersDialog
            | Event::ProvidersSelectModel { .. }
            | Event::ProvidersDisconnect { .. }
            | Event::ProvidersAdd
    )
}
