//! Panel internal helpers.
//!
//! Split from dialog/panel.rs to stay under the 500-line limit.

/// Normalize title: one leading/trailing space, trimmed empty titles stay empty.
pub fn normalize_title(title: impl Into<String>) -> String {
    let trimmed = title.into().trim().to_string();
    if trimmed.is_empty() {
        trimmed
    } else {
        format!(" {} ", trimmed)
    }
}
