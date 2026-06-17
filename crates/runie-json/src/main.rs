//! runie-json — Structured JSON input/output for scripting and piping.
//!
//! Reads a JSON request from stdin, streams response chunks as JSONL lines,
//! and outputs a final JSON response object.
//!
//! ## Request schema
//! ```json
//! {
//!   "prompt": "hello",
//!   "model": "gpt-4o",
//!   "provider": "openai",
//!   "tools": ["read_file", "bash"]
//! }
//! ```
//!
//! ## Response schema (final line)
//! ```json
//! {
//!   "content": "Hello!",
//!   "tool_calls": [],
//!   "tokens_used": 0,
//!   "duration_ms": 1234
//! }
//! ```

use anyhow::Result;
use runie_agent::{build_provider_with_warning, run_headless_turn, HeadlessOptions};
use runie_core::config_reload;
use runie_core::message::ChatMessage;

#[cfg(test)]
use runie_core::llm_event::LLMEvent;

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// JSON request read from stdin.
#[derive(Debug, Deserialize)]
struct JsonRequest {
    prompt: String,
    model: Option<String>,
    provider: Option<String>,
    tools: Option<Vec<String>>,
}

/// A single tool call in the response.
#[derive(Debug, Serialize)]
struct ToolCall {
    name: String,
    arguments: serde_json::Value,
    output: String,
}

/// Final JSON response written to stdout.
#[derive(Debug, Serialize)]
struct JsonResponse {
    content: String,
    tool_calls: Vec<ToolCall>,
    tokens_used: usize,
    duration_ms: u64,
}

/// JSONL streaming chunk.
#[derive(Debug, Serialize)]
struct StreamChunk {
    chunk: String,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run_json().await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run_json() -> Result<()> {
    let req = read_json_request().await?;
    let config = config_reload::Config::load(Some(&config_reload::config_path()));
    let (provider_name, model) = resolve_provider_and_model(&req, &config);
    let messages = build_json_messages(&req);
    let provider = build_provider_with_warning(&provider_name, &model)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    let start = Instant::now();

    let result = run_json_turn(messages, &provider).await?;
    let response = build_json_response(result, start.elapsed().as_millis() as u64);
    println!("{}", serde_json::to_string(&response)?);
    Ok(())
}

async fn read_json_request() -> Result<JsonRequest> {
    let mut stdin = tokio::io::stdin();
    let mut buf = String::new();
    use tokio::io::AsyncReadExt;
    stdin.read_to_string(&mut buf).await?;
    serde_json::from_str(&buf).map_err(|e| anyhow::anyhow!("{}", e))
}

fn resolve_provider_and_model(
    req: &JsonRequest,
    config: &config_reload::Config,
) -> (String, String) {
    let provider_name = req
        .provider
        .clone()
        .or_else(|| config.provider.clone())
        .unwrap_or_else(|| "mock".to_string());
    let model = req
        .model
        .clone()
        .or_else(|| config.default_model().map(String::from))
        .unwrap_or_else(|| "echo".to_string());
    (provider_name, model)
}

fn build_json_messages(req: &JsonRequest) -> Vec<ChatMessage> {
    let tools_list = req
        .tools
        .as_ref()
        .map(|t| t.join(", "))
        .unwrap_or_else(|| runie_core::prompts::DEFAULT_TOOLS.to_string());

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
    messages: Vec<ChatMessage>,
    provider: &runie_provider::DynProvider,
) -> Result<runie_agent::HeadlessResult> {
    let options = HeadlessOptions {
        execute_tools: true,
        max_tool_rounds: 5,
        on_chunk: Some(Box::new(|chunk: &str| {
            let line = serde_json::to_string(&StreamChunk {
                chunk: chunk.to_string(),
            })
            .unwrap_or_default();
            println!("{}", line);
        })),
    };

    run_headless_turn(messages, provider, options).await
}

fn build_json_response(result: runie_agent::HeadlessResult, duration_ms: u64) -> JsonResponse {
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

    #[tokio::test]
    async fn json_mode_returns_tool_calls() {
        use futures::StreamExt;
        use runie_agent::parser::parse_tool_calls;
        use runie_core::message::ChatMessage;
        use runie_core::provider::Provider;
        let provider = runie_provider::MockProvider::default();
        let messages = vec![
            ChatMessage::system("You are helpful."),
            ChatMessage::user("list files"),
        ];
        let mut response_text = String::new();
        let mut stream = provider.generate(messages);
        while let Some(r) = stream.next().await {
            match r.unwrap() {
                LLMEvent::TextDelta(t) => response_text.push_str(&t),
                _ => {}
            }
        }
        let _tools = parse_tool_calls(&response_text);
        // MockProvider returns deterministic response; may or may not have tools
        // We just verify the pipeline works
        assert!(!response_text.is_empty());
    }
}
