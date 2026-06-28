//! `ActorHandles` — typed registry for all production actor senders.
//!
//! This struct is the single source of truth for actor references in the runtime.
//! Each field is a typed handle to a specific actor.
//!
//! ## Usage
//!
//! ```ignore
//! // Direct access
//! handles.config.unwrap().set_theme("dark".into()).await;
//!
//! // Or use delegation helpers
//! handles.send_set_theme("dark".into()).await;
//! ```

use std::path::PathBuf;

use crate::actors::{
    RactorInputHandle, RactorIoHandle, RactorConfigHandle,
    RactorSessionHandle, RactorFffIndexerHandle, RactorTurnHandle,
    InputMsg, TurnMsg,
};
use crate::actors::provider::RactorProviderHandle;
use crate::actors::permission::RactorPermissionHandle;
use crate::model::DeliveryMode;
use crate::session::Session;
use crate::trust::TrustDecision;

/// Single registry for all production actor senders.
///
/// Each field is a typed handle to a specific actor. Callers invoke methods
/// directly on the handles rather than going through delegation helpers.
#[derive(Clone, Debug, Default)]
pub struct ActorHandles {
    /// Config actor handle.
    pub config: Option<RactorConfigHandle>,
    /// Provider actor handle.
    pub provider: Option<RactorProviderHandle>,
    /// Session actor handle.
    pub session: Option<RactorSessionHandle>,
    /// IO actor handle.
    pub io: Option<RactorIoHandle>,
    /// FFF indexer actor handle.
    pub fff_indexer: Option<RactorFffIndexerHandle>,
    /// Input actor handle.
    pub input: Option<RactorInputHandle>,
    /// Permission actor handle.
    pub permission: Option<RactorPermissionHandle>,
    /// Turn actor handle.
    pub turn: Option<RactorTurnHandle>,
}

impl ActorHandles {
    /// Create a new ActorHandles with all fields set to None.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the config handle.
    pub fn with_config(mut self, handle: RactorConfigHandle) -> Self {
        self.config = Some(handle);
        self
    }

    /// Set the provider handle.
    pub fn with_provider(mut self, handle: RactorProviderHandle) -> Self {
        self.provider = Some(handle);
        self
    }

    /// Set the session handle.
    pub fn with_session(mut self, handle: RactorSessionHandle) -> Self {
        self.session = Some(handle);
        self
    }

    /// Set the IO handle.
    pub fn with_io(mut self, handle: RactorIoHandle) -> Self {
        self.io = Some(handle);
        self
    }

    /// Set the FFF indexer handle.
    pub fn with_fff_indexer(mut self, handle: RactorFffIndexerHandle) -> Self {
        self.fff_indexer = Some(handle);
        self
    }

    /// Set the input handle.
    pub fn with_input(mut self, handle: RactorInputHandle) -> Self {
        self.input = Some(handle);
        self
    }

    /// Set the permission handle.
    pub fn with_permission(mut self, handle: RactorPermissionHandle) -> Self {
        self.permission = Some(handle);
        self
    }

    /// Set the turn handle.
    pub fn with_turn(mut self, handle: RactorTurnHandle) -> Self {
        self.turn = Some(handle);
        self
    }

    /// Returns true if all handles are present.
    pub fn is_complete(&self) -> bool {
        self.config.is_some()
            && self.provider.is_some()
            && self.session.is_some()
            && self.io.is_some()
            && self.fff_indexer.is_some()
            && self.input.is_some()
            && self.permission.is_some()
            && self.turn.is_some()
    }

    /// Returns the number of handles that are present.
    pub fn len(&self) -> usize {
        let mut count = 0;
        if self.config.is_some() { count += 1; }
        if self.provider.is_some() { count += 1; }
        if self.session.is_some() { count += 1; }
        if self.io.is_some() { count += 1; }
        if self.fff_indexer.is_some() { count += 1; }
        if self.input.is_some() { count += 1; }
        if self.permission.is_some() { count += 1; }
        if self.turn.is_some() { count += 1; }
        count
    }

    // ── Config delegation ────────────────────────────────────────────────────

    /// Send set_theme to config actor.
    pub async fn send_set_theme(&self, name: String) {
        if let Some(ref h) = self.config { h.set_theme(name).await; }
    }

    /// Send set_default_model to config actor.
    pub async fn send_set_default_model(&self, provider: &str, model: &str) {
        if let Some(ref h) = self.config {
            h.set_default_model(provider.to_owned(), model.to_owned()).await;
        }
    }

    /// Send save_provider to config actor.
    pub async fn send_save_provider(&self, name: &str, base_url: &str, api_key: &str, models: Vec<String>) {
        if let Some(ref h) = self.config {
            h.save_provider(name.to_owned(), base_url.to_owned(), api_key.to_owned(), models).await;
        }
    }

    /// Send remove_provider to config actor.
    pub async fn send_remove_provider(&self, name: &str) {
        if let Some(ref h) = self.config { h.remove_provider(name.to_owned()).await; }
    }

    /// Send set_provider_models to config actor.
    pub async fn send_set_provider_models(&self, name: &str, models: Vec<String>) {
        if let Some(ref h) = self.config { h.set_provider_models(name.to_owned(), models).await; }
    }

    /// Send set_vim_mode to config actor.
    pub async fn send_set_vim_mode(&self, enabled: bool) {
        if let Some(ref h) = self.config { h.set_vim_mode(enabled).await; }
    }

    /// Send set_telemetry to config actor.
    pub async fn send_set_telemetry(&self, enabled: bool) {
        if let Some(ref h) = self.config { h.set_telemetry(enabled).await; }
    }

    /// Send set_truncation to config actor.
    pub async fn send_set_truncation(&self, limits: crate::config::TruncationSection) {
        if let Some(ref h) = self.config { h.set_truncation(limits).await; }
    }

    /// Send set_thinking_level to config actor.
    pub async fn send_set_thinking_level(&self, level: crate::model::ThinkingLevel) {
        if let Some(ref h) = self.config { h.set_thinking_level(level).await; }
    }

    // ── Session delegation ───────────────────────────────────────────────────

    /// Send set_trust to session actor.
    pub async fn send_set_trust(&self, path: PathBuf, decision: TrustDecision) {
        if let Some(ref h) = self.session { h.set_trust(path, decision).await; }
    }

    /// Send append_history to session actor (async).
    pub async fn send_append_history(&self, entry: String) {
        if let Some(ref h) = self.session { h.append_history(entry).await; }
    }

    /// Try-send append_history (non-blocking).
    pub fn try_send_append_history(&self, entry: String) {
        if let Some(ref h) = self.session { h.try_append_history(entry); }
    }

    /// Send load_session to session actor.
    pub async fn send_load_session(&self, name: String) {
        if let Some(ref h) = self.session { h.load(name).await; }
    }

    /// Send save_session to session actor.
    pub async fn send_save_session(&self, name: String, session: Session) {
        if let Some(ref h) = self.session { h.save(name, session).await; }
    }

    /// Send delete_session to session actor.
    pub async fn send_delete_session(&self, name: String) {
        if let Some(ref h) = self.session { h.delete(name).await; }
    }

    /// Send import_session to session actor.
    pub async fn send_import_session(&self, path: PathBuf) {
        if let Some(ref h) = self.session { h.import(path).await; }
    }

    /// Send export_session to session actor.
    pub async fn send_export_session(&self, path: PathBuf, session: Session) {
        if let Some(ref h) = self.session { h.export(path, session).await; }
    }

    /// Send list_sessions to session actor.
    pub async fn send_list_sessions(&self) {
        if let Some(ref h) = self.session { h.list().await; }
    }

    // ── Input delegation ─────────────────────────────────────────────────────

    /// Try-send input message (non-blocking).
    pub fn try_send_input(&self, msg: InputMsg) {
        if let Some(ref h) = self.input { let _ = h.try_send(msg); }
    }

    // ── Permission delegation ───────────────────────────────────────────────

    /// Send resolve_permission to permission actor.
    pub async fn send_resolve_permission(&self, request_id: String, action: crate::permissions::PermissionAction) {
        if let Some(ref h) = self.permission { h.resolve_permission(request_id, action).await; }
    }

    /// Send cancel_permission to permission actor.
    pub async fn send_cancel_permission(&self, request_id: String) {
        if let Some(ref h) = self.permission { h.cancel_permission(request_id).await; }
    }

    /// Send dismiss to permission actor.
    pub async fn send_dismiss_permission(&self) {
        if let Some(ref h) = self.permission { h.dismiss().await; }
    }

    /// Try-send resolve_permission (non-blocking).
    pub fn try_resolve_permission(&self, request_id: String, action: crate::permissions::PermissionAction) {
        if let Some(ref h) = self.permission { h.try_resolve_permission(request_id, action); }
    }

    /// Try-send cancel_permission (non-blocking).
    pub fn try_cancel_permission(&self, request_id: String) {
        if let Some(ref h) = self.permission { h.try_cancel_permission(request_id); }
    }

    /// Try-send dismiss_permission (non-blocking).
    pub fn try_dismiss_permission(&self) {
        if let Some(ref h) = self.permission { h.try_dismiss(); }
    }

    // ── IO delegation ───────────────────────────────────────────────────────

    /// Send run_bash to IO actor.
    pub async fn run_bash(&self, command: String) {
        if let Some(ref h) = self.io { h.run_bash(command).await; }
    }

    /// Send write_files to IO actor.
    pub async fn write_files(&self, edits: Vec<(PathBuf, String)>) {
        if let Some(ref h) = self.io { h.write_files(edits).await; }
    }

    // ── Turn delegation ──────────────────────────────────────────────────────

    /// Send turn abort.
    pub async fn send_turn_abort(&self) {
        if let Some(ref h) = self.turn { h.send(TurnMsg::AbortTurn).await; }
    }

    /// Send turn clear_queues.
    pub async fn send_turn_clear_queues(&self) {
        if let Some(ref h) = self.turn { h.send(TurnMsg::ClearQueues).await; }
    }

    /// Send turn abort_queue.
    pub async fn send_turn_abort_queue(&self) {
        if let Some(ref h) = self.turn { h.send(TurnMsg::AbortQueue).await; }
    }

    /// Send turn queue_follow_up.
    pub async fn send_turn_queue_follow_up(&self, content: String) {
        if let Some(ref h) = self.turn {
            h.send(TurnMsg::QueueFollowUp { content }).await;
        }
    }

    /// Try-send turn queue_follow_up (non-blocking).
    pub fn try_send_turn_queue_follow_up(&self, content: String) {
        if let Some(ref h) = self.turn {
            let _ = h.try_send(TurnMsg::QueueFollowUp { content });
        }
    }

    /// Send turn deliver_queued.
    pub async fn send_turn_deliver_queued(&self, steering_mode: DeliveryMode, follow_up_mode: DeliveryMode) {
        if let Some(ref h) = self.turn {
            h.send(TurnMsg::DeliverQueued { steering_mode, follow_up_mode }).await;
        }
    }

    /// Try-send turn deliver_queued (non-blocking).
    pub fn try_send_turn_deliver_queued(&self, steering_mode: DeliveryMode, follow_up_mode: DeliveryMode) {
        if let Some(ref h) = self.turn {
            let _ = h.try_send(TurnMsg::DeliverQueued { steering_mode, follow_up_mode });
        }
    }

    /// Try-send turn submit_user_message (non-blocking).
    pub fn try_send_turn_submit_user_message(&self, content: String, id: String) {
        if let Some(ref h) = self.turn {
            let _ = h.try_send(TurnMsg::SubmitUserMessage { content, id });
        }
    }
}
