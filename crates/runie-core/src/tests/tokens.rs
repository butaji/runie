use crate::tokens::{estimate_tokens, TokenTracker};

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
