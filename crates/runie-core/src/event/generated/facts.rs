//! `is_fact_variant()` fast-path predicate.

use super::super::variants::Event;

/// Returns true if this event is a fact (not an intent or control).
///
/// Facts are broadcast state changes from actors. They do not convert to `Intent`.
/// Used by `Event::into_intent()` as a fast-path guard.
pub fn is_fact_variant(e: &Event) -> bool {
    matches!(
        e,
        // Agent
        Event::Thinking { .. }
            | Event::ThoughtDone { .. }
            | Event::ToolStart { .. }
            | Event::ToolInputDelta { .. }
            | Event::ToolEnd { .. }
            | Event::ToolConstraintError { .. }
            | Event::ResponseDelta { .. }
            | Event::ThinkingDelta { .. }
            | Event::TextStart { .. }
            | Event::TextEnd { .. }
            | Event::ThinkingStart { .. }
            | Event::ThinkingEnd { .. }
            | Event::Response { .. }
            | Event::TurnComplete { .. }
            | Event::Done { .. }
            | Event::Error { .. }
            | Event::TurnStarted { .. }
            | Event::TurnAborted
            | Event::TurnCompleted
            | Event::TurnErrored { .. }
            | Event::TurnConstraintError { .. }
            | Event::TokenStatsUpdated { .. }
            | Event::StreamStarted { .. }
            | Event::UserMessageSubmitted { .. }
            | Event::QueueAborted { .. }
            | Event::QueuesCleared
            | Event::SteeringDelivered { .. }
            | Event::FollowUpDelivered { .. }
            | Event::MessageDequeued { .. }
            | Event::IdGenerated(_)
            | Event::PermissionRequest { .. }
            | Event::PermissionRequestDismissed
            | Event::AssistantMessageReady { .. }
            // Replay
            | Event::MessageReplayed { .. }
            // Persistence
            | Event::InputChanged { .. }
            | Event::ViewChanged { .. }
            | Event::CompletionChanged { .. }
            | Event::TrustLoaded { .. }
            | Event::TrustChanged { .. }
            | Event::TrustSet { .. }
            | Event::ReadOnlyChanged { .. }
            | Event::HistoryLoaded { .. }
            | Event::HistoryAppend { .. }
            // System
            | Event::SystemMessage { .. }
            | Event::ConfigLoaded { .. }
            // IO
            | Event::GistShared { .. }
            | Event::ExternalEditorClosed { .. }
            | Event::ClipboardWritten { .. }
            | Event::ClipboardRead { .. }
            | Event::ProcessResumed
            | Event::BashOutput { .. }
            | Event::FilesWritten { .. }
            | Event::EnvDetected { .. }
            | Event::FffSearchResult { .. }
            // Session (fact variants)
            | Event::SessionLoaded { .. }
            | Event::SessionSaved { .. }
            | Event::SessionDeleted { .. }
            | Event::SessionImported { .. }
            | Event::SessionExported { .. }
            | Event::SessionList { .. }
            | Event::SessionOperationFailed { .. }
            | Event::SessionChanged { .. }
            // LoginFlow (fact variants)
            | Event::ValidationFailed { .. }
            | Event::ModelsFetched { .. }
    )
}
