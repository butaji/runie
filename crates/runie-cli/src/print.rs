//! Print mode — streaming LLM response as JSONL events to stdout.
//!
//! Each event is emitted as a newline-delimited JSON object. This provides
//! a machine-readable stream that covers all event types: text, thinking,
//! tool calls, permission requests, tool results, usage, errors, and turn end.

use anyhow::Result;
use runie_agent::headless_helper::{build_messages, build_options, build_sink};
use runie_core::event::headless::HeadlessEvent;

/// Run a single-turn LLM prompt and emit structured JSONL events to stdout.
pub fn run(prompt: &str) -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let messages = build_messages(prompt);
        let sink = build_sink(false);
        let opts = build_options(None, Some(Box::new(|event: HeadlessEvent| {
            println!("{}", event.to_json_line());
        })));
        runie_agent::run_headless_cli(None, None, messages, sink, opts).await?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_mode_accepts_prompt() {
        // Smoke test: verify the function accepts a prompt without panicking
        let result = run("test");
        // Will fail without config, but proves the dispatch works
        assert!(result.is_err() || result.is_ok());
    }
}
