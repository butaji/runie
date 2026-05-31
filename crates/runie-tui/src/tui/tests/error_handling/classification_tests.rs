//! Error classification tests.
//!
//! Tests for error recoverable classification:
//! - No API key error (recoverable=false)
//! - Invalid API key 401 (recoverable=false)
//! - Network timeout (recoverable=true)
//! - Rate limit 429 (recoverable=true)
//! - Agent timeout (10 min)

use crate::components::MessageItem;
use crate::tui::state::{AppState, TuiMode};
use crate::tui::update::agent::error::on_agent_error;
use crate::tui::tests::reducer::make_state;

/// Test helper to create error and get its recoverable flag
fn get_error_recoverable(message: &str) -> bool {
    let mut state = make_state();
    on_agent_error(&mut state, message.to_string());

    // Find the error message in state
    if let Some(MessageItem::Error { recoverable, .. }) = state.messages.last() {
        *recoverable
    } else {
        panic!("Expected error message in state");
    }
}

#[test]
fn test_error_no_api_key_is_not_recoverable() {
    let recoverable = get_error_recoverable("No API key configured for provider");
    assert!(
        !recoverable,
        "No API key error should not be recoverable"
    );
}

#[test]
fn test_error_invalid_api_key_401_is_not_recoverable() {
    let recoverable = get_error_recoverable("Invalid API key (401 Unauthorized)");
    assert!(
        !recoverable,
        "Invalid API key 401 should not be recoverable"
    );
}

#[test]
fn test_error_network_timeout_is_recoverable() {
    let recoverable = get_error_recoverable("Network timeout after 30 seconds");
    assert!(
        recoverable,
        "Network timeout should be recoverable"
    );
}

#[test]
fn test_error_connection_refused_is_recoverable() {
    let recoverable = get_error_recoverable("Connection refused by server");
    assert!(
        recoverable,
        "Connection refused should be recoverable"
    );
}

#[test]
fn test_error_rate_limit_429_is_recoverable() {
    let recoverable = get_error_recoverable("Rate limit exceeded (429)");
    assert!(
        recoverable,
        "Rate limit 429 should be recoverable"
    );
}

#[test]
fn test_error_too_many_requests_is_recoverable() {
    let recoverable = get_error_recoverable("Too many requests, please retry");
    assert!(
        recoverable,
        "Too many requests should be recoverable"
    );
}

#[test]
fn test_error_temporary_failure_is_recoverable() {
    let recoverable = get_error_recoverable("Temporary failure, please try again");
    assert!(
        recoverable,
        "Temporary failure should be recoverable"
    );
}

#[test]
fn test_error_generic_is_not_recoverable() {
    let recoverable = get_error_recoverable("Something went wrong");
    assert!(
        !recoverable,
        "Generic error should not be recoverable by default"
    );
}

#[test]
fn test_error_auth_failure_is_not_recoverable() {
    let recoverable = get_error_recoverable("Authentication failed");
    assert!(
        !recoverable,
        "Auth failure should not be recoverable"
    );
}

#[test]
fn test_error_permission_denied_is_not_recoverable() {
    let recoverable = get_error_recoverable("Permission denied");
    assert!(
        !recoverable,
        "Permission denied should not be recoverable"
    );
}

#[test]
fn test_error_timeout_in_message_is_recoverable() {
    // Test various timeout patterns
    assert!(get_error_recoverable("Request timeout"), "timeout should be recoverable");
    assert!(get_error_recoverable("Connection timed out"), "timed out should be recoverable");
    assert!(get_error_recoverable("Operation timed out after 30s"), "timed out should be recoverable");
}

#[test]
fn test_error_network_in_message_is_recoverable() {
    // Test various network error patterns
    assert!(get_error_recoverable("Network error occurred"), "network should be recoverable");
    assert!(get_error_recoverable("Network connection lost"), "network should be recoverable");
    assert!(get_error_recoverable("network is unreachable"), "network should be recoverable");
}

#[test]
fn test_error_case_insensitive_matching() {
    // Test case insensitive matching for recoverable patterns
    assert!(get_error_recoverable("TIMEOUT"), "TIMEOUT should match");
    assert!(get_error_recoverable("Network"), "Network should match");
    assert!(get_error_recoverable("RATE LIMIT"), "RATE LIMIT should match");
    assert!(get_error_recoverable("CONNECTION REFUSED"), "CONNECTION REFUSED should match");
}
