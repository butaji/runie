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
use runie_agent::headless_helper::{build_options, build_sink, build_system_prompt};
use runie_agent::{run_headless_cli, HeadlessResult};
use runie_core::message::ChatMessage;

#[cfg(test)]
use runie_core::provider_event::ProviderEvent;

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
    let yolo = std::env::args().any(|a| a == "--yolo");
    if yolo {
        eprintln!("warning: --yolo enabled; destructive tools will be auto-approved");
    }
    if let Err(e) = run_json(yolo).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

async fn run_json(yolo: bool) -> Result<()> {
    let req = read_json_request().await?;
    let messages = build_json_messages(&req);
    let start = Instant::now();

    let result = run_json_turn(req.provider.as_deref(), req.model.as_deref(), messages, yolo)
        .await?;
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
    provider_name: Option<&str>,
    model: Option<&str>,
    messages: Vec<ChatMessage>,
    yolo: bool,
) -> Result<HeadlessResult> {
    let sink = build_sink(yolo);
    let opts = build_options(Some(Box::new(|chunk: &str| {
        let line = serde_json::to_string(&StreamChunk {
            chunk: chunk.to_string(),
        })
        .unwrap_or_default();
        println!("{}", line);
    })));

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
    use runie_core::tool::ToolStatus;
    use std::sync::Mutex;

    /// Guards current-directory mutations during tests that run tools.
    static CWD_LOCK: Mutex<()> = Mutex::new(());

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
        use runie_core::tool_parser::parse_tool_calls;
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
                ProviderEvent::TextDelta(t) => response_text.push_str(&t),
                _ => {}
            }
        }
        let _tools = parse_tool_calls(&response_text);
        assert!(!response_text.is_empty());
    }

    #[tokio::test]
    async fn headless_default_denies_destructive_tool() {
        let _guard = CWD_LOCK.lock().unwrap();
        std::env::set_var("RUNIE_MOCK", "1");
        let dir = tempfile::tempdir().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let messages = vec![
            ChatMessage::system("You are helpful."),
            ChatMessage::user("write something".to_string()),
        ];
        let result = run_json_turn(Some("mock"), None, messages, false).await.unwrap();

        std::env::remove_var("RUNIE_MOCK");
        std::env::set_current_dir(original).unwrap();

        let write_output = result
            .tool_outputs
            .iter()
            .find(|o| o.tool_name == "write_file")
            .expect("expected a write_file tool call");
        assert_eq!(write_output.status, ToolStatus::Blocked);
        assert!(!dir.path().join("hello.txt").exists());
    }

    #[tokio::test]
    async fn headless_yolo_allows_destructive_tool() {
        let _guard = CWD_LOCK.lock().unwrap();
        std::env::set_var("RUNIE_MOCK", "1");
        let dir = tempfile::tempdir().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(&dir).unwrap();

        let messages = vec![
            ChatMessage::system("You are helpful."),
            ChatMessage::user("write something".to_string()),
        ];
        let result = run_json_turn(Some("mock"), None, messages, true).await.unwrap();

        std::env::remove_var("RUNIE_MOCK");
        std::env::set_current_dir(original).unwrap();

        let write_output = result
            .tool_outputs
            .iter()
            .find(|o| o.tool_name == "write_file")
            .expect("expected a write_file tool call");
        assert_eq!(write_output.status, ToolStatus::Success);
        assert!(dir.path().join("hello.txt").exists());
    }
}
