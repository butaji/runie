use crate::tokens::{
    estimate_tokens, estimate_tokens_for_model, estimate_tokens_with_tokenizer, token_tracker_for,
    TokenTracker, Tokenizer,
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
fn tiktoken_counts_openai_model() {
    let text = "日本語テキスト";
    let approx = estimate_tokens(text);
    let tiktoken = estimate_tokens_with_tokenizer(text, Tokenizer::Tiktoken("o200k_base".into()));
    assert!(
        tiktoken > approx,
        "tiktoken count {tiktoken} should exceed approximation {approx}"
    );
}

#[test]
fn approximate_fallback_for_unknown_model() {
    let text = "hello world";
    assert_eq!(
        estimate_tokens_with_tokenizer(text, Tokenizer::Tiktoken("unknown_base".into())),
        estimate_tokens(text)
    );
}

#[test]
fn token_tracker_uses_real_counts() {
    let text = "Hello world";
    let mut tracker =
        TokenTracker::with_costs(0.0, 0.0).with_tokenizer(Tokenizer::Tiktoken("o200k_base".into()));
    tracker.track_input(text);
    assert_ne!(
        tracker.input_total(),
        estimate_tokens(text),
        "tracker should use tiktoken counts, not chars/4 approximation"
    );
}

#[test]
fn token_tracker_uses_registry_costs() {
    let tracker = token_tracker_for("openai", "gpt-4o");
    assert_eq!(tracker.session_cost(), 0.0);
    assert!(
        matches!(tracker.tokenizer(), Tokenizer::Tiktoken(name) if name == "o200k_base"),
        "gpt-4o should use o200k_base tokenizer"
    );
}

#[test]
fn estimate_tokens_selects_model_tokenizer() {
    let text = "Hello world";
    let model_count = estimate_tokens_for_model(text, "openai", "gpt-4o");
    let approx = estimate_tokens(text);
    assert_ne!(
        model_count, approx,
        "gpt-4o should use tiktoken, not chars/4 approximation"
    );
}

#[test]
fn unknown_model_falls_back_to_approximation() {
    let text = "Hello world";
    assert_eq!(
        estimate_tokens_for_model(text, "unknown", "unknown"),
        estimate_tokens(text)
    );
}
