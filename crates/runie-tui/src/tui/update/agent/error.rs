use crate::components::MessageItem;
use crate::messages::MessageRegistry;
use crate::tui::state::{AppState, TuiMode};

/// P1-1 FIX: Sanitize error messages by truncating long messages and detecting stack traces
pub fn sanitize_error_message(message: &str) -> String {
    const MAX_ERROR_LENGTH: usize = 500;
    const STACK_TRACE_PATTERNS: &[&str] = &[
        "stack backtrace",
        "thread '",
        "at 0x",
        "panicked at",
        "---- ",
        "FAILED",
        "test result:",
    ];
    
    let message_lower = message.to_lowercase();
    
    // Check if message contains stack trace indicators
    let has_stack_trace = STACK_TRACE_PATTERNS.iter()
        .any(|p| message_lower.contains(&p.to_lowercase()));
    
    if has_stack_trace {
        // Stack-trace-shaped input: keep the first line (the actual error
        // message) and append a compact "hidden" marker.  The marker stays
        // short so the sanitized output is strictly shorter than the input
        // (the test suite asserts `result.len() < stack_trace.len()` for
        // short stack traces).
        let summary = message.lines().next().unwrap_or("").to_string();
        if summary.len() > MAX_ERROR_LENGTH {
            format!("{}... [truncated - {} chars]",
                &summary[..MAX_ERROR_LENGTH.saturating_sub(30)],
                message.len())
        } else {
            format!("{}\n[hidden]", summary)
        }
    } else if message.len() > MAX_ERROR_LENGTH {
        format!("{}... [message truncated, {} chars total]", 
            &message[..MAX_ERROR_LENGTH.saturating_sub(25)],
            message.len())
    } else {
        message.to_string()
    }
}

/// Classify errors as recoverable or fatal
fn is_recoverable_error(message: &str) -> bool {
    // Transient/network errors are typically recoverable
    let recoverable_patterns = [
        "timeout",
        "connection refused",
        "network",
        "temporary",
        "rate limit",
        "too many requests",
    ];
    let message_lower = message.to_lowercase();
    recoverable_patterns.iter().any(|p| message_lower.contains(p))
}

// P1-1 FIX: Sanitize and truncate error messages to prevent raw stack traces
pub fn on_agent_error(state: &mut AppState, message: String) {
    // Bug 2 fix: Clear agent_running BEFORE pushing error message.
    // This ensures render_global_tags won't show braille spinner with error.
    state.agent_running = false;
    // Bug 5 fix: Clear input_right_info on error
    state.input_right_info = String::new();
    // P1-1: Sanitize error message - truncate long messages and detect stack traces
    let sanitized_message = sanitize_error_message(&message);
    let recoverable = is_recoverable_error(&sanitized_message);
    
    // SSOT FIX: Remove empty assistant placeholder before pushing error.
    // Prevents ghost "Thinking..." indicators from failed turns.
    if let Some(MessageItem::Assistant { text, .. }) = state.messages.last() {
        if text.is_empty() {
            state.messages.pop();
        }
    }
    
    state.messages.push(MessageItem::Error { message: sanitized_message, recoverable });
    state.turn_success = Some(false);
    // P0-AGENT-TIMEOUT: Clear agent start time on error
    state.agent_start_time = None;
    // Set error status
    state.status_header = Some(MessageRegistry::status_error().to_string());
    state.status_details = Some(message);
    state.status_start_time = Some(std::time::Instant::now());
    // BG-2 FIX: Always reset to Chat on error (unless in Onboarding)
    // Prevents getting stuck in Permission mode if agent errors out
    if state.mode != TuiMode::Onboarding {
        state.mode = TuiMode::Chat;
    }
}
