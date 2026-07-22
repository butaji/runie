//! Print mode — streaming LLM response as JSONL events to stdout.
//!
//! Each event is emitted as a newline-delimited JSON object. This provides
//! a machine-readable stream that covers all event types: text, thinking,
//! tool calls, permission requests, tool results, usage, errors, and turn end.

use anyhow::Result;
use runie_agent::headless_helper::{build_messages, build_options, build_sink};
use runie_core::event::headless::HeadlessEvent;

/// Run a single-turn LLM prompt and emit structured JSONL events to stdout.
pub async fn run(prompt: &str, sandbox: bool) -> Result<()> {
    // Set sandbox environment variable if enabled
    if sandbox {
        std::env::set_var("RUNIE_SANDBOX", "1");
    }

    let messages = build_messages(prompt);
    // Auto-allow tools in print mode (yolo=true) so the CLI is useful out of the box.
    // The config.toml [permissions] section can still restrict specific tools.
    let sink = build_sink(true);
    let opts = build_options(
        None,
        Some(Box::new(|event: HeadlessEvent| {
            println!("{}", event.to_json_line());
        })),
    );
    runie_agent::run_headless_cli(None, None, messages, sink, opts, None).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use runie_agent::headless::{run_headless_turn, HeadlessOptions};
    use runie_core::event::headless::HeadlessEvent;
    use runie_core::message::ChatMessage;
    use runie_core::permissions::{AutoAllowSink, PermissionManager};
    use runie_provider::MockProvider;
    use std::sync::{Arc, Mutex};

    /// Smoke: run_headless_cli produces HeadlessEvents via the on_event callback.
    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn print_mode_emits_jsonl_events() {
        // Capture HeadlessEvents emitted during the turn.
        let events: Arc<Mutex<Vec<HeadlessEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let captured = events.clone();

        let messages = vec![ChatMessage::system("You are helpful."), ChatMessage::user("say hello")];

        let provider = MockProvider::default();
        let sink: Arc<dyn runie_core::permissions::ApprovalSink> = Arc::new(AutoAllowSink);

        let options = HeadlessOptions {
            execute_tools: false,
            max_tool_rounds: 5,
            on_chunk: None,
            on_event: Some(Box::new(move |evt: HeadlessEvent| {
                captured.lock().unwrap().push(evt);
            })),
            permission_gate: runie_agent::PermissionGate::new(PermissionManager::default(), sink.clone()),
        };

        run_headless_turn(messages, &provider, options)
            .await
            .expect("turn should succeed");

        let events = events.lock().unwrap();
        // The mock provider emits text, so we expect at least one Text event.
        assert!(
            events
                .iter()
                .any(|e| matches!(e, HeadlessEvent::Text { .. })),
            "expected at least one Text event, got: {:?}",
            events
        );
        // Must emit End to mark the stream as finished.
        assert!(
            events
                .iter()
                .any(|e| matches!(e, HeadlessEvent::End { .. })),
            "expected an End event, got: {:?}",
            events
        );
        // Every event must round-trip through JSONL serialization.
        for evt in events.iter() {
            let line = evt.to_json_line();
            let parsed: HeadlessEvent = serde_json::from_str(&line)
                .unwrap_or_else(|e| panic!("HeadlessEvent failed to round-trip as JSONL: {e}, line: {line}"));
            // Variants must match after round-trip.
            let evt_ref = evt;
            let same_variant = std::mem::discriminant(evt_ref) == std::mem::discriminant(&parsed);
            assert!(
                same_variant,
                "JSONL round-trip variant mismatch for {evt:?}, line: {line}"
            );
        }
    }

    /// Verify print mode's `run()` function accepts a prompt (smoke).
    #[tokio::test]
    async fn print_mode_run_smoke() {
        // We cannot easily capture stdout in a test, so just verify `run` doesn't panic.
        let _ = run("hello", false).await;
    }
}
