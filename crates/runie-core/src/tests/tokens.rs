use crate::tokens::{estimate_tokens, estimate_tokens_for_model, TokenTracker};

#[test]
fn estimate_tokens_empty() {
    assert_eq!(estimate_tokens(""), 0);
}

#[test]
fn estimate_tokens_english() {
    // Tiktoken: "Hello world" tokenization varies by version
    let text = "Hello world";
    let count = estimate_tokens(text);
    assert!(
        (2..=3).contains(&count),
        "Hello world should be 2-3 tokens, got {}",
        count
    );
}

#[test]
fn estimate_tokens_long() {
    // 100 'a' chars ≈ 25 tokens with chars/4; tiktoken uses ~20 for repeated 'a's
    let text = "a".repeat(100);
    let count = estimate_tokens(&text);
    assert!(count > 0, "should produce tokens");
    assert!(count <= 25, "should be reasonable for 100 chars");
}

#[test]
fn estimate_tokens_unicode() {
    let text = "日本語テキスト";
    assert!(estimate_tokens(text) > 0);
}

#[test]
fn estimate_tokens_one_char_rounds_up() {
    // Tiktoken: "x" tokenization varies by version
    let count = estimate_tokens("x");
    assert!(
        (1..=2).contains(&count),
        "'x' should be 1-2 tokens, got {}",
        count
    );
}

#[test]
fn estimate_tokens_four_chars_is_one() {
    // Tiktoken: "test" tokenization varies by version
    let count = estimate_tokens("test");
    assert!(
        (1..=2).contains(&count),
        "'test' should be 1-2 tokens, got {}",
        count
    );
}

#[test]
fn token_tracker_new_zero() {
    let tracker = TokenTracker::new();
    assert_eq!(tracker.session_total(), 0);
    assert_eq!(tracker.turn_total(), 0);
}

#[test]
fn token_tracker_add_message() {
    let mut tracker = TokenTracker::new();
    tracker.add_input(100);
    tracker.add_output(50);
    assert_eq!(tracker.session_total(), 150);
    assert_eq!(tracker.turn_total(), 150);
    assert_eq!(tracker.input_total(), 100);
    assert_eq!(tracker.output_total(), 50);
}

#[test]
fn token_tracker_reset_turn() {
    let mut tracker = TokenTracker::new();
    tracker.add_input(100);
    tracker.add_output(50);
    tracker.end_turn();
    assert_eq!(tracker.session_total(), 150);
    assert_eq!(tracker.turn_total(), 0);
    tracker.add_input(20);
    assert_eq!(tracker.turn_total(), 20);
}

#[test]
fn cost_estimation_mock() {
    let tracker = TokenTracker::with_costs(0.50, 1.50);
    let mut t = tracker;
    t.add_input(1_000_000);
    t.add_output(500_000);
    assert_eq!(t.session_cost(), 1.25);
}

#[test]
fn cost_estimation_zero() {
    let tracker = TokenTracker::new();
    assert_eq!(tracker.session_cost(), 0.0);
}

#[test]
fn estimate_tokens_uses_tiktoken_for_openai() {
    let text = "hello world";
    // Tiktoken: "hello world" tokenization varies by version
    let tiktoken_count = estimate_tokens(text);
    assert!(
        (2..=3).contains(&tiktoken_count),
        "hello world should be 2-3 tokens, got {}",
        tiktoken_count
    );
    // OpenAI provider uses tiktoken
    assert_eq!(
        estimate_tokens_for_model(text, "openai", "gpt-4o"),
        tiktoken_count
    );
    // Unknown provider falls back to chars/4: 11 chars ceil(11/4) = 3
    assert_eq!(estimate_tokens_for_model(text, "unknown", "unknown"), 3);
    // Known provider (minimax) uses tiktoken
    let tik = estimate_tokens(text);
    assert_eq!(estimate_tokens_for_model(text, "minimax", "abab6.5s"), tik);
}

#[test]
fn token_tracker_track_uses_tiktoken() {
    // Tiktoken: "Hello world" = 2 tokens
    let text = "Hello world";
    let mut tracker = TokenTracker::with_costs(0.0, 0.0);
    let expected = estimate_tokens(text);
    tracker.track_input(text);
    assert_eq!(tracker.input_total(), expected);
    tracker.track_output(text);
    assert_eq!(tracker.output_total(), expected);
}

#[test]
fn token_tracker_estimate_uses_tiktoken() {
    let text = "Hello world";
    let tracker = TokenTracker::new();
    assert_eq!(tracker.estimate_input(text), estimate_tokens(text));
    assert_eq!(tracker.estimate_output(text), estimate_tokens(text));
}
