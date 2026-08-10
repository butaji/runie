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
