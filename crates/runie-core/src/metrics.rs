//! Metrics facade for Runie telemetry.
//!
//! Uses the `metrics` crate to emit counters, gauges, and histograms for:
//! - model_switch: when the user or system switches provider/model
//! - tool_usage: when a tool is invoked
//! - token_counts: input/output token estimates
//!
//! Recording is gated by `telemetry_enabled()` config. When disabled, a
//! no-op recorder is installed so `counter!()` etc. are still safe to call.

use std::sync::OnceLock;

/// Global flag to track if metrics have been initialized.
static INITIALIZED: OnceLock<()> = OnceLock::new();

/// Initialize metrics with a no-op recorder.
///
/// Call this early in `main()` or binary startup. When telemetry is enabled,
/// replace with a real exporter (e.g., `metrics_exporter_prometheus`).
pub fn init() {
    if INITIALIZED.get().is_some() {
        return;
    }

    // Install no-op recorder. Replace with `metrics::set_global_recorder(...)` + a real
    // exporter (prometheus, opendelemetry, etc.) when observability is needed.
    metrics::set_global_recorder(metrics::NoopRecorder).ok();
    let _ = INITIALIZED.set(());
}

/// Record a model/provider switch event.
#[inline]
pub fn record_model_switch(provider: &str, model: &str) {
    let name = String::from("runie.model_switch");
    let labels = vec![(String::from("provider"), provider.to_owned()), (String::from("model"), model.to_owned())];
    let counter = ::metrics::counter!(name, &labels);
    counter.increment(1);
}

/// Record a tool invocation event.
#[inline]
pub fn record_tool_usage(tool_name: &str) {
    let name = String::from("runie.tool_usage");
    let labels = vec![(String::from("tool"), tool_name.to_owned())];
    let counter = ::metrics::counter!(name, &labels);
    counter.increment(1);
}

/// Record input token count estimate.
#[inline]
pub fn record_input_tokens(provider: &str, model: &str, tokens: u64) {
    let name = String::from("runie.input_tokens");
    let labels = vec![(String::from("provider"), provider.to_owned()), (String::from("model"), model.to_owned())];
    let hist = ::metrics::histogram!(name, &labels);
    hist.record(tokens as f64);
}

/// Record output token count estimate.
#[inline]
pub fn record_output_tokens(provider: &str, model: &str, tokens: u64) {
    let name = String::from("runie.output_tokens");
    let labels = vec![(String::from("provider"), provider.to_owned()), (String::from("model"), model.to_owned())];
    let hist = ::metrics::histogram!(name, &labels);
    hist.record(tokens as f64);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_model_switch_does_not_panic() {
        // Init for test context (may already be initialized)
        init();
        record_model_switch("openai", "gpt-4o-mini");
        record_model_switch("minimax", "abab6.5s-chat");
    }

    #[test]
    fn record_tool_usage_does_not_panic() {
        init();
        record_tool_usage("bash");
        record_tool_usage("read_file");
        record_tool_usage("write_file");
    }

    #[test]
    fn record_token_counts_do_not_panic() {
        init();
        record_input_tokens("openai", "gpt-4o-mini", 100);
        record_output_tokens("openai", "gpt-4o-mini", 50);
    }

    #[test]
    fn metrics_init_is_idempotent() {
        // Calling init twice should not panic
        init();
        init();
        // Should still be able to record metrics
        record_model_switch("test", "model");
        record_tool_usage("bash");
    }

    #[test]
    fn record_with_various_provider_models() {
        init();
        record_model_switch("openai", "gpt-4o");
        record_model_switch("openai", "gpt-4o-mini");
        record_model_switch("anthropic", "claude-3-5-sonnet");
        record_model_switch("minimax", "abab6.5s-chat");
    }
}
