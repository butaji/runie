//! Subagent effect handler.

use runie_core::model::ThinkingLevel;
use runie_core::Event as CoreEvent;
use tokio::sync::mpsc;

/// Run the subagent asynchronously and emit a `SystemMessage` when done.
pub fn run(
    prompt: String,
    provider: String,
    model: String,
    thinking: ThinkingLevel,
    read_only: bool,
    skills_context: String,
    tx: mpsc::Sender<CoreEvent>,
) {
    let preview = truncate_preview(&prompt, 60);
    tokio::spawn(async move {
        let result = runie_agent::subagent::run_subagent(
            &prompt,
            &provider,
            &model,
            thinking,
            read_only,
            &skills_context,
            "",
            5,
        )
        .await;
        let msg = format_result(&preview, result);
        let _ = tx
            .send(CoreEvent::SystemMessage { content: msg })
            .await;
    });
}

fn truncate_preview(text: &str, max: usize) -> String {
    let preview: String = text.chars().take(max).collect();
    if text.chars().count() > max {
        format!("{}…", preview)
    } else {
        preview
    }
}

fn format_result(
    preview: &str,
    result: Result<String, runie_agent::subagent::SubagentError>,
) -> String {
    match result {
        Ok(text) => {
            let snippet = truncate_preview(&text, 200);
            format!("Subagent \"{}\" → {}", preview, snippet)
        }
        Err(e) => format!("Subagent \"{}\" failed: {}", preview, e),
    }
}
