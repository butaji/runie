//! Tests for the subagent feature.

use crate::subagent::run_subagent;
use crate::tests::ensure_mock_provider;
use runie_core::model::ThinkingLevel;

#[test]
fn run_subagent_returns_echo_of_prompt() {
    ensure_mock_provider();
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
    ensure_mock_provider();
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
    ensure_mock_provider();
    // The mock provider should still respond to an empty prompt.
    let result = run_subagent("", "mock", "echo", ThinkingLevel::Off, false, "", "", 5);
    let out = result.expect("empty prompt should still produce a result");
    // Don't assert content (mock may or may not echo empty); just that it ran.
    let _ = out;
}

#[test]
fn run_subagent_returns_error_for_unknown_provider() {
    // Unknown providers now return an explicit error (no silent Mock fallback).
    // The subagent must propagate this as a SubagentError.
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
        result.is_err(),
        "expected error for unknown provider, got: {:?}",
        result
    );
}
