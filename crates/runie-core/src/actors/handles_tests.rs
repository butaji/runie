//! Tests for `ActorHandles` (typed actor handle registry).
//!
//! `ActorHandles` is an alias for `LeaderHandle`. These tests verify the
//! alias relationship and that `LeaderHandle` exposes all expected fields.

#[cfg(test)]
mod tests {
    use crate::actors::leader::test_leader_handle;
    use crate::actors::LeaderHandle;

    // ── Layer 1: State/Logic ────────────────────────────────────────────────

    /// Verify `ActorHandles` is `LeaderHandle` (aliased).
    #[tokio::test]
    async fn actor_handles_is_leader_handle() {
        let handle: LeaderHandle = test_leader_handle().await;
        // The `ActorHandles` alias should resolve to `LeaderHandle`.
        let _: crate::actors::ActorHandles = handle;
    }

    /// Verify `LeaderHandle` has all expected typed actor fields.
    #[tokio::test]
    async fn leader_handle_has_all_actor_fields() {
        let handle = test_leader_handle().await;
        // Config
        let _: &crate::actors::RactorConfigHandle = &handle.config;
        // Provider
        let _: &crate::actors::RactorProviderHandle = &handle.provider;
        // Session
        let _: &crate::actors::RactorSessionHandle = &handle.session;
        // IO
        let _: &crate::actors::RactorIoHandle = &handle.io;
        // Permission
        let _: &crate::actors::RactorPermissionHandle = &handle.permission;
        // Turn
        let _: &crate::actors::RactorTurnHandle = &handle.turn;
        // Input
        let _: &crate::actors::RactorInputHandle = &handle.input;
        // FFF indexer
        let _: &crate::actors::RactorFffIndexerHandle = &handle.fff_indexer;
        // Agent (dyn trait)
        let _: &std::sync::Arc<dyn crate::actors::leader::LeaderAgentHandle> = &handle.agent;
    }

    // ── Layer 2: Event Handling ─────────────────────────────────────────────

    /// Verify we can send a message to the config actor via the leader handle.
    #[tokio::test]
    async fn leader_config_send_reaches_actor() {
        let handle = test_leader_handle().await;
        use crate::actors::ConfigMsg;
        handle.config.send_message(ConfigMsg::Reload).await;
    }

    /// Verify we can send a message to the session actor via the leader handle.
    #[tokio::test]
    async fn leader_session_send_reaches_actor() {
        let handle = test_leader_handle().await;
        use crate::actors::SessionMsg;
        handle.session.send_message(SessionMsg::List).await;
    }

    /// Verify we can send a message to the turn actor via the leader handle.
    #[tokio::test]
    async fn leader_turn_send_reaches_actor() {
        let handle = test_leader_handle().await;
        use crate::actors::TurnMsg;
        handle.turn.send_message(TurnMsg::ClearQueues).await;
    }
}
