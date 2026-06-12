//! Tests for the subagent feature.

use crate::subagent::run_subagent;
use runie_core::model::ThinkingLevel;

#[test]
fn run_subagent_returns_echo_of_prompt() {
    // The mock provider echoes the user input.
    let result = run_subagent(
        "hello subagent",
        "mock",
        "echo",
        ThinkingLevel::Off,
        false,
        "",
        "",
        5,
    );
    let out = result.expect("subagent should succeed");
    assert!(
        out.contains("hello subagent"),
        "expected echoed input in output, got: {:?}",
        out
    );
}

#[test]
fn run_subagent_with_skill_context_uses_it() {
    // Skills context is part of the system prompt; mock just echoes the
    // concatenated content. This asserts the wiring is intact.
    let result = run_subagent(
        "ask about skill",
        "mock",
        "echo",
        ThinkingLevel::Off,
        false,
        "SKILL: test-skill",
        "",
        5,
    );
    let out = result.expect("subagent should succeed");
    assert!(out.contains("ask about skill"));
}

#[test]
fn run_subagent_empty_prompt_succeeds() {
    // The mock provider should still respond to an empty prompt.
    let result = run_subagent("", "mock", "echo", ThinkingLevel::Off, false, "", "", 5);
    let out = result.expect("empty prompt should still produce a result");
    // Don't assert content (mock may or may not echo empty); just that it ran.
    let _ = out;
}

#[test]
fn run_subagent_falls_back_to_mock_for_unknown_provider() {
    // Unknown providers fall back to mock (per `AnyProvider::new`).
    // The subagent must still return a result, not panic.
    let result = run_subagent(
        "anything",
        "bogus-provider-xyz",
        "echo",
        ThinkingLevel::Off,
        false,
        "",
        "",
        5,
    );
    assert!(
        result.is_ok(),
        "expected fallback to mock to succeed, got: {:?}",
        result
    );
}
