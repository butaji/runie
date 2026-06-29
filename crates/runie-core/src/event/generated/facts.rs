//! `is_fact_variant()` fast-path predicate.
//! Generated from `taxonomy.json`. DO NOT EDIT.

use super::super::variants::Event;

/// Returns true if this event is a fact (not an intent or control).
pub fn is_fact_variant(e: &Event) -> bool {
    matches!(
        e,
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
        | Event::IdGenerated { .. }
        | Event::AssistantMessageReady { .. }
        | Event::GistShared { .. }
        | Event::ExternalEditorClosed { .. }
        | Event::ClipboardWritten { .. }
        | Event::ClipboardRead { .. }
        | Event::ProcessResumed
        | Event::BashOutput { .. }
        | Event::FilesWritten { .. }
        | Event::EnvDetected { .. }
        | Event::FffSearchResult { .. }
        | Event::MessageReplayed { .. }
        | Event::InputChanged { .. }
        | Event::ViewChanged { .. }
        | Event::CompletionChanged { .. }
        | Event::TrustLoaded { .. }
        | Event::TrustChanged { .. }
        | Event::TrustSet { .. }
        | Event::ReadOnlyChanged { .. }
        | Event::HistoryLoaded { .. }
        | Event::HistoryAppend { .. }
        | Event::TransientMessage { .. }
        | Event::TransientError { .. }
        | Event::ClearTransient
        | Event::ShowDiagnostics
        | Event::SystemMessage { .. }
        | Event::ConfigLoaded { .. }
    )
}
