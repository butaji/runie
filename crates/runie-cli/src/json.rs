//! JSON mode — structured JSON stdin/stdout for scripting.
//!
//! Reads a JSON request from stdin, streams response chunks as JSONL lines
//! using the unified HeadlessEvent format, and outputs a final JSON response object.

use anyhow::Result;
use runie_agent::headless_helper::{build_options, build_sink};
use runie_agent::{run_headless_cli, HeadlessResult};
use runie_core::event::headless::HeadlessEvent;
use runie_core::message::ChatMessage;
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::time::Instant;

/// JSON request read from stdin.
#[derive(Debug, Deserialize)]
struct JsonRequest {
    prompt: String,
    model: Option<String>,
    provider: Option<String>,
    tools: Option<Vec<String>>,
}

/// Final JSON response written to stdout.
#[derive(Debug, Serialize)]
struct JsonResponse {
    content: String,
    tool_calls: Vec<ToolCall>,
    tokens_used: usize,
    duration_ms: u64,
}

/// A single tool call in the response.
#[derive(Debug, Serialize)]
struct ToolCall {
    name: String,
    arguments: serde_json::Value,
    output: String,
}

/// Run JSON mode: read request from stdin, stream chunks as JSONL, output final JSON.
pub async fn run() -> Result<()> {
    let req = read_json_request()?;
    let messages = build_json_messages(&req);
    let start = Instant::now();

    let result = run_json_turn(req.provider.as_deref(), req.model.as_deref(), messages).await?;

    let response = build_json_response(result, start.elapsed().as_millis() as u64);
    println!("{}", serde_json::to_string(&response)?);
    Ok(())
}

fn read_json_request() -> Result<JsonRequest> {
    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    serde_json::from_str(&buf).map_err(|e| anyhow::anyhow!("{}", e))
}

fn build_json_messages(req: &JsonRequest) -> Vec<ChatMessage> {
    let tools_list = req
        .tools
        .as_ref()
        .map(|t| t.join(", "))
        .unwrap_or_else(|| runie_core::prompts::DEFAULT_TOOLS.to_owned());

    let system = runie_core::prompts::build_system_prompt(
        runie_core::prompts::DEFAULT_PROMPT,
        &tools_list,
        false,
        "",
    );

    vec![
        ChatMessage::system(system),
        ChatMessage::user(req.prompt.clone()),
    ]
}

async fn run_json_turn(
    provider_name: Option<&str>,
    model: Option<&str>,
    messages: Vec<ChatMessage>,
) -> Result<HeadlessResult> {
    let sink = build_sink(false);
    let opts = build_options(
        None,
        Some(Box::new(|event: HeadlessEvent| {
            println!("{}", event.to_json_line());
        })),
    );

    run_headless_cli(provider_name, model, messages, sink, opts).await
}

fn build_json_response(result: HeadlessResult, duration_ms: u64) -> JsonResponse {
    let tool_calls: Vec<ToolCall> = result
        .tool_outputs
        .iter()
        .map(|output| ToolCall {
            name: output.tool_name.clone(),
            arguments: output.tool_args.clone(),
            output: output.content.clone(),
        })
        .collect();
    JsonResponse {
        content: result.content.clone(),
        tool_calls,
        tokens_used: runie_core::tokens::estimate_tokens(&result.content),
        duration_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_mode_parses_request() {
        let json = r#"{"prompt": "hello", "model": "gpt-4o"}"#;
        let req: JsonRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.prompt, "hello");
        assert_eq!(req.model, Some("gpt-4o".to_string()));
    }

    #[test]
    fn json_mode_outputs_valid_json() {
        let resp = JsonResponse {
            content: "hi".into(),
            tool_calls: vec![],
            tokens_used: 1,
            duration_ms: 100,
        };
        let s = serde_json::to_string(&resp).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed["content"], "hi");
    }
}
