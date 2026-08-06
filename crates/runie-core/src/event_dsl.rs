//! Declarative event helpers shared by replay and integration harnesses.

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
        }
    }};
}

#[macro_export]
macro_rules! assistant_event_kind {
    ($event:expr) => {{
        match $event {
            $crate::types::AssistantMessageEvent::Start => "Start",
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
    }

    #[test]
    fn assistant_event_kind_macro_covers_event_families() {
        assert_eq!(
            crate::assistant_event_kind!(AssistantMessageEvent::Start),
            "Start"
        );
        assert_eq!(
            crate::assistant_event_kind!(AssistantMessageEvent::TextDelta {
                index: 0,
                delta: "x".into()
            }),
            "TextDelta"
        );
        assert_eq!(
            crate::assistant_event_kind!(AssistantMessageEvent::Error {
                error: "failed".into(),
                message: None
            }),
            "Error"
        );
    }
}
