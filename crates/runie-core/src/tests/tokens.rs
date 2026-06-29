use crate::tokens::{
    estimate_tokens, estimate_tokens_for_model, estimate_tokens_with_tokenizer,
    TokenTracker,
};

#[test]
fn estimate_tokens_empty() {
    assert_eq!(estimate_tokens(""), 0);
}

#[test]
fn estimate_tokens_english() {
    let text = "Hello world";
    assert_eq!(estimate_tokens(text), 3);
}

#[test]
fn estimate_tokens_long() {
    let text = "a".repeat(100);
    assert_eq!(estimate_tokens(&text), 25);
}

#[test]
fn estimate_tokens_unicode() {
    let text = "日本語テキスト";
    assert!(estimate_tokens(text) > 0);
}

#[test]
fn estimate_tokens_one_char_rounds_up() {
    assert_eq!(estimate_tokens("x"), 1);
}

#[test]
fn estimate_tokens_four_chars_is_one() {
    assert_eq!(estimate_tokens("test"), 1);
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
fn estimate_tokens_uses_heuristic() {
    let text = "hello world";
    // All estimation uses the chars/4 heuristic
    assert_eq!(estimate_tokens_with_tokenizer(text), estimate_tokens(text));
    assert_eq!(
        estimate_tokens_for_model(text, "openai", "gpt-4o"),
        estimate_tokens(text)
    );
    assert_eq!(
        estimate_tokens_for_model(text, "unknown", "unknown"),
        estimate_tokens(text)
    );
}

#[test]
fn token_tracker_track_uses_heuristic() {
    let text = "Hello world";
    let mut tracker = TokenTracker::with_costs(0.0, 0.0);
    let expected = estimate_tokens(text);
    tracker.track_input(text);
    assert_eq!(tracker.input_total(), expected);
    tracker.track_output(text);
    assert_eq!(tracker.output_total(), expected);
}

#[test]
fn token_tracker_estimate_uses_heuristic() {
    let text = "Hello world";
    let tracker = TokenTracker::new();
    assert_eq!(tracker.estimate_input(text), estimate_tokens(text));
    assert_eq!(tracker.estimate_output(text), estimate_tokens(text));
}
