//! Declarative event helpers shared by replay and integration harnesses.

/// Declare a closed producer-side enum whose only wire conversion is explicit.
/// This keeps event payload construction readable while preventing free-form
/// record-type strings from leaking into actor boundaries.
#[macro_export]
macro_rules! wire_kind {
    ($vis:vis enum $name:ident { $($variant:ident => $wire:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        $vis enum $name {
            $($variant),+
        }

        impl $name {
            const fn wire_name(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire),+
                }
            }
        }
    };
}

/// Return the stable kind name used by YAML expectations and event oracles.
///
/// The expansion is intentionally a plain readable match: it centralizes
/// naming without hiding event payloads or ordering.
#[macro_export]
macro_rules! agent_event_kind {
    ($event:expr) => {{
        match $event {
            $crate::types::AgentEvent::AgentStart => "AgentStart",
            $crate::types::AgentEvent::AgentEnd { .. } => "AgentEnd",
            $crate::types::AgentEvent::Error { .. } => "Error",
            $crate::types::AgentEvent::ThinkingLevelChanged { .. } => "ThinkingLevelChanged",
            $crate::types::AgentEvent::Reset => "Reset",
            $crate::types::AgentEvent::TurnStart => "TurnStart",
            $crate::types::AgentEvent::Waiting { .. } => "Waiting",
            $crate::types::AgentEvent::ThemeChanged { .. } => "ThemeChanged",
            $crate::types::AgentEvent::ModelChanged { .. } => "ModelChanged",
            $crate::types::AgentEvent::ActiveToolsChanged { .. } => "ActiveToolsChanged",
            $crate::types::AgentEvent::SessionLabelChanged { .. } => "SessionLabelChanged",
            $crate::types::AgentEvent::BranchSummaryCreated { .. } => "BranchSummaryCreated",
            $crate::types::AgentEvent::CustomSessionEntryCreated { .. } => {
                "CustomSessionEntryCreated"
            }
            $crate::types::AgentEvent::CompactionCreated { .. } => "CompactionCreated",
            $crate::types::AgentEvent::OperationRecordCreated { .. } => "OperationRecordCreated",
            $crate::types::AgentEvent::ToolDisplayModeChanged { .. } => "ToolDisplayModeChanged",
            $crate::types::AgentEvent::TurnEnd { .. } => "TurnEnd",
            $crate::types::AgentEvent::MessageStart { .. } => "MessageStart",
            $crate::types::AgentEvent::MessageUpdate { .. } => "MessageUpdate",
            $crate::types::AgentEvent::MessageEnd { .. } => "MessageEnd",
            $crate::types::AgentEvent::ToolExecutionStart { .. } => "ToolExecutionStart",
            $crate::types::AgentEvent::ToolExecutionUpdate { .. } => "ToolExecutionUpdate",
            $crate::types::AgentEvent::ToolExecutionEnd { .. } => "ToolExecutionEnd",
            $crate::types::AgentEvent::BackgroundWorkStarted { .. } => "BackgroundWorkStarted",
            $crate::types::AgentEvent::BackgroundWorkProgress { .. } => "BackgroundWorkProgress",
            $crate::types::AgentEvent::BackgroundWorkFinished { .. } => "BackgroundWorkFinished",
            $crate::types::AgentEvent::BackgroundWorkCancelled { .. } => "BackgroundWorkCancelled",
            $crate::types::AgentEvent::WorkflowStarted { .. } => "WorkflowStarted",
            $crate::types::AgentEvent::WorkflowProgress { .. } => "WorkflowProgress",
            $crate::types::AgentEvent::WorkflowFinished { .. } => "WorkflowFinished",
        }
    }};
}

/// Construct a Pi session-lane event with an explicit compile-time family.
/// The payload remains JSON because Pi's wire record fields differ per family;
/// the macro prevents unknown record-type strings at Rust call sites.
#[macro_export]
macro_rules! session_lane_event {
    ($kind:ident, $data:expr) => {
        $crate::types::AgentEvent::OperationRecordCreated {
            record_type: $crate::session_lane_record_name!($kind).to_owned(),
            data: $data,
        }
    };
}

#[macro_export]
macro_rules! session_lane_record_name {
    (operation_started) => {
        "operation_started"
    };
    (abort_requested) => {
        "abort_requested"
    };
    (operation_finished) => {
        "operation_finished"
    };
    (step_attempt) => {
        "step_attempt"
    };
    (tool_started) => {
        "tool_started"
    };
    (queue_enqueued) => {
        "queue_enqueued"
    };
    (queue_cancelled) => {
        "queue_cancelled"
    };
    (write_deferred) => {
        "write_deferred"
    };
    (usage) => {
        "usage"
    };
}

#[macro_export]
macro_rules! assistant_event_kind {
    ($event:expr) => {{
        match $event {
            $crate::types::AssistantMessageEvent::Start { .. } => "Start",
            $crate::types::AssistantMessageEvent::TextStart { .. } => "TextStart",
            $crate::types::AssistantMessageEvent::TextDelta { .. } => "TextDelta",
            $crate::types::AssistantMessageEvent::TextEnd { .. } => "TextEnd",
            $crate::types::AssistantMessageEvent::ThinkingStart { .. } => "ThinkingStart",
            $crate::types::AssistantMessageEvent::ThinkingDelta { .. } => "ThinkingDelta",
            $crate::types::AssistantMessageEvent::ThinkingEnd { .. } => "ThinkingEnd",
            $crate::types::AssistantMessageEvent::ToolCallStart { .. } => "ToolCallStart",
            $crate::types::AssistantMessageEvent::ToolCallDelta { .. } => "ToolCallDelta",
            $crate::types::AssistantMessageEvent::ToolCallEnd { .. } => "ToolCallEnd",
            $crate::types::AssistantMessageEvent::Done { .. } => "Done",
            $crate::types::AssistantMessageEvent::Error { .. } => "Error",
        }
    }};
}

/// Build an owned event sequence for compact Rust-side helpers.
///
/// YAML remains the preferred no-recompile fixture format; this macro is for
/// small typed tests and adapters that already have event expressions. It has
/// no reducer or side effects: the caller still transfers each event through
/// the owning actor/bus.
#[macro_export]
macro_rules! event_sequence {
    ($($event:expr),* $(,)?) => {
        ::std::vec![$($event),*]
    };
    ($event:expr; $count:expr) => {
        ::std::vec![$event; $count]
    };
}

/// Declare a compact telemetry replay sequence for typed Rust adapters.
///
/// YAML remains the no-recompile format for scenario coverage; this macro is
/// intentionally limited to empty-attribute lifecycle commands so it cannot
/// hide actor ownership or reducer behavior.
#[macro_export]
macro_rules! telemetry_replay {
    (
        $(start $name:literal $(parent $parent:expr)?;)*
        $(event $event_id:literal $event_name:literal;)*
        $(status $status_id:literal $status:ident;)*
        $(end $end_id:literal;)*
    ) => {
        ::std::vec![
            $(
                $crate::telemetry::TelemetryAction::Start {
                    parent_id: $crate::telemetry_parent_id!($($parent)?),
                    name: $name.to_owned(),
                    attributes: ::std::collections::HashMap::new(),
                }
            ),*,
            $(
                $crate::telemetry::TelemetryAction::Event {
                    id: $event_id,
                    name: $event_name.to_owned(),
                    attributes: ::std::collections::HashMap::new(),
                }
            ),*,
            $(
                $crate::telemetry::TelemetryAction::Status {
                    id: $status_id,
                    status: $crate::telemetry::SpanStatus::$status,
                    error: None,
                }
            ),*,
            $(
                $crate::telemetry::TelemetryAction::End { id: $end_id }
            ),*
        ]
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! telemetry_parent_id {
    ($parent:expr) => {
        Some($parent)
    };
    () => {
        None
    };
}

/// Declare a small string-backed action registry without hand-written label
/// matching. The expansion remains a plain enum plus readable `from_label`
/// match, so callers can inspect it with `cargo expand`.
#[macro_export]
macro_rules! typed_action_registry {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $( $variant:ident => $label:literal ),+ $(,)?
    }) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq)]
        $vis enum $name {
            $( $variant ),+
        }

        impl $name {
            $vis const fn labels() -> &'static [&'static str] {
                &[$($label),+]
            }

            $vis fn from_label(label: &str) -> Option<Self> {
                Some(match label {
                    $( $label => Self::$variant, )+
                    _ => return None,
                })
            }

            $vis fn filtered_labels(query: &str) -> Vec<&'static str> {
                let query = query.to_ascii_lowercase();
                Self::labels()
                    .iter()
                    .copied()
                    .filter(|entry| {
                        query.is_empty() || entry.to_ascii_lowercase().contains(&query)
                    })
                    .collect()
            }

            $vis fn selected_label(query: &str, selected: usize) -> Option<&'static str> {
                Self::filtered_labels(query).into_iter().nth(selected)
            }

            $vis fn entry_count(query: &str) -> usize {
                Self::filtered_labels(query).len()
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::types::AssistantMessageEvent;

    crate::typed_action_registry! {
        enum TestAction {
            First => "First",
            Second => "Second",
        }
    }

    #[test]
    fn typed_action_registry_maps_labels_and_rejects_unknown_values() {
        assert_eq!(TestAction::labels(), &["First", "Second"]);
        assert_eq!(TestAction::from_label("First"), Some(TestAction::First));
        assert_eq!(TestAction::from_label("missing"), None);
        assert_eq!(TestAction::filtered_labels("sec"), ["Second"]);
        assert_eq!(TestAction::selected_label("", 1), Some("Second"));
        assert_eq!(TestAction::entry_count("first"), 1);
    }

    #[test]
    fn assistant_event_kind_macro_covers_event_families() {
        assert_eq!(
            crate::assistant_event_kind!(AssistantMessageEvent::Start {
                partial: crate::types::AssistantMessage::default()
            }),
            "Start"
        );
        assert_eq!(
            crate::assistant_event_kind!(AssistantMessageEvent::TextDelta {
                index: 0,
                delta: "x".into(),
                partial: crate::types::AssistantMessage::default()
            }),
            "TextDelta"
        );
        assert_eq!(
            crate::assistant_event_kind!(AssistantMessageEvent::Error {
                reason: crate::types::StopReason::Error,
                error: crate::types::AssistantMessage::with_error(
                    crate::types::StopReason::Error,
                    "failed",
                )
            }),
            "Error"
        );
    }

    #[test]
    fn event_sequence_macro_only_constructs_owned_values() {
        let events = crate::event_sequence![
            crate::types::AgentEvent::AgentStart,
            crate::types::AgentEvent::TurnStart,
        ];
        assert!(matches!(
            events.as_slice(),
            [
                crate::types::AgentEvent::AgentStart,
                crate::types::AgentEvent::TurnStart,
            ]
        ));

        let repeated = crate::event_sequence![crate::types::AgentEvent::Reset; 2];
        assert_eq!(repeated.len(), 2);
    }

    #[test]
    fn session_lane_macro_covers_every_pi_record_family() {
        let events = vec![
            crate::session_lane_event!(operation_started, serde_json::json!({"id": "op"})),
            crate::session_lane_event!(abort_requested, serde_json::json!({"runId": "op"})),
            crate::session_lane_event!(operation_finished, serde_json::json!({"runId": "op"})),
            crate::session_lane_event!(step_attempt, serde_json::json!({"runId": "op"})),
            crate::session_lane_event!(tool_started, serde_json::json!({"runId": "op"})),
            crate::session_lane_event!(queue_enqueued, serde_json::json!({"runId": "op"})),
            crate::session_lane_event!(queue_cancelled, serde_json::json!({"id": "queue"})),
            crate::session_lane_event!(write_deferred, serde_json::json!({"runId": "op"})),
            crate::session_lane_event!(usage, serde_json::json!({"entryId": "entry"})),
        ];
        assert_eq!(events.len(), 9);
        assert!(matches!(
            &events[0],
            crate::types::AgentEvent::OperationRecordCreated { record_type, .. }
                if record_type == "operation_started"
        ));
        assert!(matches!(
            &events[8],
            crate::types::AgentEvent::OperationRecordCreated { record_type, .. }
                if record_type == "usage"
        ));
    }

    #[test]
    fn telemetry_replay_macro_constructs_declarative_actions() {
        let actions = crate::telemetry_replay![
            start "run";
            start "child" parent 0;
            event 1 "finished";
            status 1 Ok;
            end 1;
            end 0;
        ];
        assert!(matches!(
            actions[1],
            crate::telemetry::TelemetryAction::Start {
                parent_id: Some(0),
                ..
            }
        ));
        assert!(matches!(
            actions[3],
            crate::telemetry::TelemetryAction::Status {
                status: crate::telemetry::SpanStatus::Ok,
                ..
            }
        ));
    }
}
