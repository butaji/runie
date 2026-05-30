use crate::components::MessageItem;
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
        // Extract just the first line(s) for stack traces - the error summary
        let lines: Vec<&str> = message.lines()
            .take(5)  // Take first 5 lines as summary
            .collect();
        
        let summary = lines.join("\n");
        if summary.len() > MAX_ERROR_LENGTH {
            format!("{}... [truncated - {} chars total]", 
                &summary[..MAX_ERROR_LENGTH.saturating_sub(30)],
                message.len())
        } else {
            format!("{}\n[Additional details hidden. Run with --verbose for full output.]", summary)
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
    // P1-1: Sanitize error message - truncate long messages and detect stack traces
    let sanitized_message = sanitize_error_message(&message);
    let recoverable = is_recoverable_error(&sanitized_message);
    state.messages.push(MessageItem::Error { message: sanitized_message, recoverable });
    state.agent_running = false;
    // P0-AGENT-TIMEOUT: Clear agent start time on error
    state.agent_start_time = None;
    // Set error status
    state.status_header = Some("Error".to_string());
    state.status_details = Some(message);
    state.status_start_time = Some(std::time::Instant::now());
    // BG-2 FIX: Always reset to Chat on error (unless in Onboarding)
    // Prevents getting stuck in Permission mode if agent errors out
    if state.mode != TuiMode::Onboarding {
        state.mode = TuiMode::Chat;
    }
}
