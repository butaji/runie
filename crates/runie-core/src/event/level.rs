//! Severity level for transient notifications.

/// Severity level for transient notifications shown in the hints line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TransientLevel {
    Info,
    Success,
    Warning,
    Error,
}
