//! Shared StopReason projections for telemetry and the TUI.

macro_rules! stop_reason_names {
    ($(($reason:ident, $telemetry:literal, $display:literal)),+ $(,)?) => {
        impl crate::types::StopReason {
            pub const fn telemetry_name(self) -> &'static str {
                match self { $(Self::$reason => $telemetry,)+ }
            }

            pub const fn display_name(self) -> &'static str {
                match self { $(Self::$reason => $display,)+ }
            }
        }
    };
}

stop_reason_names! {
    (Stop, "stop", "stop"),
    (ToolUse, "tool_use", "toolUse"),
    (MaxTokens, "length", "length"),
    (Error, "error", "error"),
    (Aborted, "aborted", "aborted"),
    (Pending, "pending", "pending"),
    (Deferred, "deferred", "deferred"),
}

impl crate::types::StopReason {
    /// Normalize the closed provider finish vocabulary once at the domain
    /// boundary. Unknown terminal values fail closed; absent values preserve
    /// the legacy tool-call fallback used by streaming adapters.
    pub fn from_provider_finish_reason(raw: Option<&str>, has_tool_calls: bool) -> Self {
        let Some(raw) = raw else {
            return if has_tool_calls {
                Self::ToolUse
            } else {
                Self::Stop
            };
        };
        match raw {
            "stop" | "end_turn" | "completed" => Self::Stop,
            "length" | "max_tokens" | "max_output_tokens" => Self::MaxTokens,
            "tool_calls" | "tool_use" => Self::ToolUse,
            "aborted" | "cancelled" => Self::Aborted,
            "error" | "failed" | "incomplete" | "content_filter" | "filtered" => Self::Error,
            _ => Self::Error,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::types::StopReason;

    #[test]
    fn provider_finish_reason_mapping_is_shared_and_fails_closed() {
        assert_eq!(
            StopReason::from_provider_finish_reason(Some("max_output_tokens"), false),
            StopReason::MaxTokens
        );
        assert_eq!(
            StopReason::from_provider_finish_reason(Some("unknown_provider_state"), false),
            StopReason::Error
        );
        assert_eq!(
            StopReason::from_provider_finish_reason(None, true),
            StopReason::ToolUse
        );
    }
}
