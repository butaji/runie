//! `ActorHandles` — single registry for all actor senders.

use std::path::PathBuf;

use crate::actors::{
    CompletionActorHandle, ConfigActorHandle, FffSearchRequest, InputActorHandle,
    IoActorHandle, PermissionActorHandle, ProviderActorHandle, SessionActorHandle,
    TrustActorHandle, TurnActorHandle, ViewActorHandle,
};
use crate::config::TruncationSection;
use crate::model::ThinkingLevel;
use crate::session::Session;
use crate::trust::TrustDecision;

#[derive(Clone, Debug, Default)]
pub struct ActorHandles {
    pub config: Option<ConfigActorHandle>,
    pub provider: Option<ProviderActorHandle>,
    pub session: Option<SessionActorHandle>,
    pub io: Option<IoActorHandle>,
    pub fff_indexer: Option<FffIndexerHandle>,
    pub input: Option<InputActorHandle>,
    pub permission: Option<PermissionActorHandle>,
    pub view: Option<ViewActorHandle>,
    pub completion: Option<CompletionActorHandle>,
    pub trust: Option<TrustActorHandle>,
    pub turn: Option<TurnActorHandle>,
}

#[derive(Clone, Debug)]
pub struct FffIndexerHandle {
    tx: tokio::sync::mpsc::Sender<FffSearchRequest>,
}

impl FffIndexerHandle {
    pub fn new(tx: tokio::sync::mpsc::Sender<FffSearchRequest>) -> Self {
        Self { tx }
    }
    pub async fn search(&self, request: FffSearchRequest) {
        let _ = self.tx.send(request).await;
    }
    pub fn try_search(&self, request: FffSearchRequest) {
        let _ = self.tx.try_send(request);
    }
}

impl ActorHandles {
    // Config helpers
    pub async fn send_set_default_model(&self, provider: &str, model: &str) {
        if let Some(ref h) = self.config {
            h.set_default_model(provider.to_owned(), model.to_owned()).await;
        }
    }

    pub async fn send_save_provider(&self, name: &str, base_url: &str, api_key: &str, models: Vec<String>) {
        if let Some(ref h) = self.config {
            h.save_provider(name.to_owned(), base_url.to_owned(), api_key.to_owned(), models).await;
        }
    }

    pub async fn send_remove_provider(&self, name: &str) {
        if let Some(ref h) = self.config { h.remove_provider(name.to_owned()).await; }
    }

    pub async fn send_set_provider_models(&self, name: &str, models: Vec<String>) {
        if let Some(ref h) = self.config { h.set_provider_models(name.to_owned(), models).await; }
    }

    pub async fn send_set_theme(&self, name: String) {
        if let Some(ref h) = self.config { h.set_theme(name).await; }
    }

    pub async fn send_set_vim_mode(&self, enabled: bool) {
        if let Some(ref h) = self.config { h.set_vim_mode(enabled).await; }
    }

    pub async fn send_set_telemetry(&self, enabled: bool) {
        if let Some(ref h) = self.config { h.set_telemetry(enabled).await; }
    }

    pub async fn send_set_truncation(&self, limits: TruncationSection) {
        if let Some(ref h) = self.config { h.set_truncation(limits).await; }
    }

    pub async fn send_set_thinking_level(&self, level: ThinkingLevel) {
        if let Some(ref h) = self.config { h.set_thinking_level(level).await; }
    }

    // Session helpers
    pub async fn send_set_trust(&self, path: PathBuf, decision: TrustDecision) {
        if let Some(ref h) = self.session { h.set_trust(path, decision).await; }
    }

    pub async fn send_trust(&self, path: PathBuf, decision: TrustDecision) {
        if let Some(ref h) = self.trust {
            h.send(crate::actors::TrustMsg::SetTrust { path, decision }).await;
        }
    }

    pub async fn send_init_read_only(&self, path: PathBuf) {
        if let Some(ref h) = self.trust {
            h.send(crate::actors::TrustMsg::InitReadOnly { path }).await;
        }
    }

    pub async fn send_append_history(&self, entry: String) {
        if let Some(ref h) = self.session { h.append_history(entry).await; }
    }

    pub fn try_send_append_history(&self, entry: String) {
        if let Some(ref h) = self.session { h.try_append_history(entry); }
    }

    pub async fn send_load_session(&self, name: String) {
        if let Some(ref h) = self.session { h.load(name).await; }
    }

    pub async fn send_save_session(&self, name: String, session: Session) {
        if let Some(ref h) = self.session { h.save(name, session).await; }
    }

    pub async fn send_delete_session(&self, name: String) {
        if let Some(ref h) = self.session { h.delete(name).await; }
    }

    pub async fn send_list_sessions(&self) {
        if let Some(ref h) = self.session { h.list().await; }
    }

    pub async fn send_import_session(&self, path: PathBuf) {
        if let Some(ref h) = self.session { h.import(path).await; }
    }

    pub async fn send_export_session(&self, path: PathBuf, session: Session) {
        if let Some(ref h) = self.session { h.export(path, session).await; }
    }

    // Session state mutations
    pub fn try_send_session_add_user(&self, content: String, images: Vec<String>) {
        if let Some(ref h) = self.session { h.try_add_user_message(content, images); }
    }

    pub fn try_send_session_add_system(&self, content: String) {
        if let Some(ref h) = self.session { h.try_add_system_message(content); }
    }

    pub fn try_send_session_add_tool(&self, id: String, name: String, content: String) {
        if let Some(ref h) = self.session { h.try_add_tool_message(id, name, content); }
    }

    pub fn try_send_session_update_tool(&self, id_contains: String, content: String) {
        if let Some(ref h) = self.session { h.try_update_tool_message(id_contains, content); }
    }

    pub fn try_send_session_add_turn_complete(&self, id: String, content: String) {
        if let Some(ref h) = self.session { h.try_add_turn_complete(id, content); }
    }

    pub fn try_send_session_add_error(&self, id: String, content: String) {
        if let Some(ref h) = self.session { h.try_add_error_message(id, content); }
    }

    pub fn try_send_session_reset(&self) {
        if let Some(ref h) = self.session { h.try_reset(); }
    }

    pub fn try_send_session_fork_at(&self, index: usize) {
        if let Some(ref h) = self.session { h.try_fork_at(index); }
    }

    pub fn try_send_session_clone_branch(&self) {
        if let Some(ref h) = self.session { h.try_clone_branch(); }
    }

    pub fn try_send_session_push_pending_edit(&self, edit: crate::edit_preview::EditPreview) {
        if let Some(ref h) = self.session { h.try_push_pending_edit(edit); }
    }

    pub fn try_send_session_drain_pending_edits(&self) {
        if let Some(ref h) = self.session { h.try_drain_pending_edits(); }
    }

    pub fn try_send_session_clear_pending_edits(&self) {
        if let Some(ref h) = self.session { h.try_clear_pending_edits(); }
    }

    // IO helpers
    pub async fn send_fff_search(&self, request: FffSearchRequest) {
        if let Some(ref h) = self.fff_indexer { h.search(request).await; }
    }

    pub async fn send_input(&self, msg: crate::actors::InputMsg) {
        if let Some(ref h) = self.input { h.send(msg).await; }
    }

    pub fn try_send_input(&self, msg: crate::actors::InputMsg) {
        if let Some(ref h) = self.input { h.try_send(msg); }
    }

    pub async fn send_resolve_permission(&self, request_id: String, action: crate::permissions::PermissionAction) {
        if let Some(ref h) = self.permission { h.resolve_permission(request_id, action).await; }
    }

    pub async fn send_cancel_permission(&self, request_id: String) {
        if let Some(ref h) = self.permission { h.cancel_permission(request_id).await; }
    }

    pub async fn send_dismiss_permission(&self) {
        if let Some(ref h) = self.permission { h.dismiss().await; }
    }

    pub fn try_resolve_permission(&self, request_id: String, action: crate::permissions::PermissionAction) {
        if let Some(ref h) = self.permission { h.try_resolve_permission(request_id, action); }
    }

    pub fn try_cancel_permission(&self, request_id: String) {
        if let Some(ref h) = self.permission { h.try_cancel_permission(request_id); }
    }

    pub fn try_dismiss_permission(&self) {
        if let Some(ref h) = self.permission { h.try_dismiss(); }
    }

    pub async fn send_view(&self, msg: crate::actors::ViewMsg) {
        if let Some(ref h) = self.view { h.send(msg).await; }
    }

    pub fn try_send_view(&self, msg: crate::actors::ViewMsg) {
        if let Some(ref h) = self.view { h.try_send(msg); }
    }

    pub async fn run_bash(&self, command: String) {
        if let Some(ref h) = self.io { h.run_bash(command).await; }
    }

    pub async fn write_files(&self, edits: Vec<(PathBuf, String)>) {
        if let Some(ref h) = self.io { h.write_files(edits).await; }
    }

    // TurnActor helpers
    pub async fn send_turn_run_if_queued(&self) {
        if let Some(ref h) = self.turn { h.send(crate::actors::TurnMsg::RunIfQueued).await; }
    }

    pub async fn send_turn_abort(&self) {
        if let Some(ref h) = self.turn { h.send(crate::actors::TurnMsg::AbortTurn).await; }
    }

    pub async fn send_turn_clear_queues(&self) {
        if let Some(ref h) = self.turn { h.send(crate::actors::TurnMsg::ClearQueues).await; }
    }

    pub fn try_send_turn_abort(&self) {
        if let Some(ref h) = self.turn { h.try_send(crate::actors::TurnMsg::AbortTurn); }
    }

    pub fn try_send_turn_clear_queues(&self) {
        if let Some(ref h) = self.turn { h.try_send(crate::actors::TurnMsg::ClearQueues); }
    }

    // Queue helpers
    pub async fn send_turn_queue_steering(&self, content: String) {
        if let Some(ref h) = self.turn {
            h.send(crate::actors::TurnMsg::QueueSteering { content }).await;
        }
    }

    pub fn try_send_turn_queue_steering(&self, content: String) {
        if let Some(ref h) = self.turn {
            h.try_send(crate::actors::TurnMsg::QueueSteering { content });
        }
    }

    pub async fn send_turn_queue_follow_up(&self, content: String) {
        if let Some(ref h) = self.turn {
            h.send(crate::actors::TurnMsg::QueueFollowUp { content }).await;
        }
    }

    pub fn try_send_turn_queue_follow_up(&self, content: String) {
        if let Some(ref h) = self.turn {
            h.try_send(crate::actors::TurnMsg::QueueFollowUp { content });
        }
    }

    pub async fn send_turn_abort_queue(&self) {
        if let Some(ref h) = self.turn {
            h.send(crate::actors::TurnMsg::AbortQueue).await;
        }
    }

    pub fn try_send_turn_abort_queue(&self) {
        if let Some(ref h) = self.turn {
            h.try_send(crate::actors::TurnMsg::AbortQueue);
        }
    }

    pub async fn send_turn_submit_user_message(&self, content: String, id: String) {
        if let Some(ref h) = self.turn {
            h.send(crate::actors::TurnMsg::SubmitUserMessage { content, id }).await;
        }
    }

    pub fn try_send_turn_submit_user_message(&self, content: String, id: String) {
        if let Some(ref h) = self.turn {
            h.try_send(crate::actors::TurnMsg::SubmitUserMessage { content, id });
        }
    }

    // Turn lifecycle helpers
    pub async fn send_turn_thinking(&self, id: String) {
        if let Some(ref h) = self.turn {
            h.send(crate::actors::TurnMsg::Thinking { id }).await;
        }
    }

    pub fn try_send_turn_thinking(&self, id: String) {
        if let Some(ref h) = self.turn {
            h.try_send(crate::actors::TurnMsg::Thinking { id });
        }
    }

    pub async fn send_turn_tool_start(&self, id: String, name: String) {
        if let Some(ref h) = self.turn {
            h.send(crate::actors::TurnMsg::ToolStart { id, name }).await;
        }
    }

    pub fn try_send_turn_tool_start(&self, id: String, name: String) {
        if let Some(ref h) = self.turn {
            h.try_send(crate::actors::TurnMsg::ToolStart { id, name });
        }
    }

    pub async fn send_turn_tool_end(&self, id: String, duration_secs: f64, output: String) {
        if let Some(ref h) = self.turn {
            h.send(crate::actors::TurnMsg::ToolEnd { id, duration_secs, output }).await;
        }
    }

    pub fn try_send_turn_tool_end(&self, id: String, duration_secs: f64, output: String) {
        if let Some(ref h) = self.turn {
            h.try_send(crate::actors::TurnMsg::ToolEnd { id, duration_secs, output });
        }
    }

    pub async fn send_turn_response_delta(&self, id: String, content: String) {
        if let Some(ref h) = self.turn {
            h.send(crate::actors::TurnMsg::ResponseDelta { id, content }).await;
        }
    }

    pub fn try_send_turn_response_delta(&self, id: String, content: String) {
        if let Some(ref h) = self.turn {
            h.try_send(crate::actors::TurnMsg::ResponseDelta { id, content });
        }
    }

    pub async fn send_turn_complete(&self, id: String, duration_secs: f64) {
        if let Some(ref h) = self.turn {
            h.send(crate::actors::TurnMsg::TurnComplete { id, duration_secs }).await;
        }
    }

    pub fn try_send_turn_complete(&self, id: String, duration_secs: f64) {
        if let Some(ref h) = self.turn {
            h.try_send(crate::actors::TurnMsg::TurnComplete { id, duration_secs });
        }
    }

    pub async fn send_turn_done(&self, id: String) {
        if let Some(ref h) = self.turn {
            h.send(crate::actors::TurnMsg::Done { id }).await;
        }
    }

    pub fn try_send_turn_done(&self, id: String) {
        if let Some(ref h) = self.turn {
            h.try_send(crate::actors::TurnMsg::Done { id });
        }
    }

    pub async fn send_turn_error(&self, id: String, message: String) {
        if let Some(ref h) = self.turn {
            h.send(crate::actors::TurnMsg::Error { id, message }).await;
        }
    }

    pub fn try_send_turn_error(&self, id: String, message: String) {
        if let Some(ref h) = self.turn {
            h.try_send(crate::actors::TurnMsg::Error { id, message });
        }
    }

    pub async fn send_turn_update_speed(&self, tokens_out: usize) {
        if let Some(ref h) = self.turn {
            h.send(crate::actors::TurnMsg::UpdateSpeed { tokens_out }).await;
        }
    }

    pub fn try_send_turn_update_speed(&self, tokens_out: usize) {
        if let Some(ref h) = self.turn {
            h.try_send(crate::actors::TurnMsg::UpdateSpeed { tokens_out });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_handles_default_is_all_none() {
        let handles = ActorHandles::default();
        assert!(handles.config.is_none());
        assert!(handles.provider.is_none());
        assert!(handles.session.is_none());
        assert!(handles.io.is_none());
        assert!(handles.fff_indexer.is_none());
        assert!(handles.input.is_none());
        assert!(handles.permission.is_none());
        assert!(handles.view.is_none());
        assert!(handles.completion.is_none());
        assert!(handles.trust.is_none());
        assert!(handles.turn.is_none());
    }

    #[test]
    fn fff_indexer_handle_is_cloneable() {
        fn _assert_clone<T: Clone>() {}
        _assert_clone::<FffIndexerHandle>();
    }

    #[tokio::test]
    async fn actor_handles_send_save_provider_via_actor() {
        use crate::actors::ConfigActor;
        use crate::bus::EventBus;
        use crate::Event;

        let bus = EventBus::<Event>::new(16);
        let (handle, _actor) = ConfigActor::spawn(bus.clone(), None);
        let mut handles = ActorHandles::default();
        handles.config = Some(handle);
        handles.send_save_provider("test", "http://localhost", "key", vec!["model".into()]).await;
        drop(handles);
        drop(_actor);
    }
}
