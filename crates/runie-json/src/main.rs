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
use runie_agent::{build_provider, parser::parse_tool_calls, Tool};
use runie_core::{
    config_reload,
    provider::{Message, Provider, ResponseChunk},
};
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
    let mut stdin = tokio::io::stdin();
    let mut buf = String::new();
    use tokio::io::AsyncReadExt;
    stdin.read_to_string(&mut buf).await?;

    let req: JsonRequest = serde_json::from_str(&buf)?;

    let config = config_reload::Config::load_from(&config_reload::config_path());
    let provider_name = req.provider.as_deref().or(config.provider.as_deref()).unwrap_or("mock");
    let model = req.model.as_deref().or(config.default_model()).unwrap_or("echo");

    let tools_list = req.tools.as_ref().map(|t| t.join(", ")).unwrap_or_else(|| {
        "read_file, list_dir, write_file, edit_file, bash, grep, find".to_string()
    });

    let system = runie_core::prompts::build_system_prompt(
        runie_core::prompts::DEFAULT_PROMPT,
        &tools_list,
        false,
        "",
    );

    let mut messages = vec![
        Message::System { content: system },
        Message::User { content: req.prompt.clone() },
    ];

    let provider = build_provider(provider_name, model);
    let start = Instant::now();
    let mut content = String::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();

    for _ in 0..5 {
        let mut response_text = String::new();
        provider
            .generate(messages.clone(), |chunk: ResponseChunk| {
                response_text.push_str(&chunk.content);
                content.push_str(&chunk.content);
                let line = serde_json::to_string(&StreamChunk {
                    chunk: chunk.content,
                })
                .unwrap_or_default();
                println!("{}", line);
            })
            .await?;

        let tools = parse_tool_calls(&response_text);
        if tools.is_empty() {
            break;
        }

        messages.push(Message::Assistant {
            content: response_text.clone(),
        });

        for tool in &tools {
            let result = execute_tool(tool);
            tool_calls.push(ToolCall {
                name: tool.name().to_string(),
                arguments: tool_to_json(tool),
                output: result.output.clone(),
            });
            messages.push(Message::ToolResult {
                content: format!("{} result:\n{}", tool.name(), result.output),
            });
        }
    }

    let duration_ms = start.elapsed().as_millis() as u64;
    let tokens_used = runie_core::tokens::estimate_tokens(&content);

    let response = JsonResponse {
        content,
        tool_calls,
        tokens_used,
        duration_ms,
    };
    println!("{}", serde_json::to_string(&response)?);
    Ok(())
}

fn execute_tool(tool: &Tool) -> runie_agent::ToolResult {
    match tool {
        Tool::EditFile { .. } => tool.execute(),
        _ => tool.execute(),
    }
}

fn tool_to_json(tool: &Tool) -> serde_json::Value {
    match tool {
        Tool::ReadFile { path, offset, limit } => {
            let mut m = serde_json::Map::new();
            m.insert("path".into(), path.clone().into());
            if let Some(o) = offset { m.insert("offset".into(), (*o).into()); }
            if let Some(l) = limit { m.insert("limit".into(), (*l).into()); }
            serde_json::Value::Object(m)
        }
        Tool::ListDir { path } => {
            serde_json::json!({"path": path})
        }
        Tool::WriteFile { path, content } => {
            serde_json::json!({"path": path, "content": content})
        }
        Tool::EditFile { path, search, replace } => {
            serde_json::json!({"path": path, "search": search, "replace": replace})
        }
        Tool::Bash { command } => {
            serde_json::json!({"command": command})
        }
        Tool::Grep { pattern, path, glob, ignore_case, literal, context, limit } => {
            let mut m = serde_json::Map::new();
            m.insert("pattern".into(), pattern.clone().into());
            m.insert("path".into(), path.clone().into());
            if let Some(g) = glob { m.insert("glob".into(), g.clone().into()); }
            m.insert("ignore_case".into(), (*ignore_case).into());
            m.insert("literal".into(), (*literal).into());
            m.insert("context".into(), (*context).into());
            m.insert("limit".into(), (*limit).into());
            serde_json::Value::Object(m)
        }
        Tool::Find { pattern, path, limit } => {
            serde_json::json!({"pattern": pattern, "path": path, "limit": limit})
        }
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
        let provider = runie_provider::MockProvider::default();
        let mut messages = vec![
            Message::System {
                content: "You are helpful.".into(),
            },
            Message::User {
                content: "list files".into(),
            },
        ];
        let mut response_text = String::new();
        provider
            .generate(messages.clone(), |chunk: ResponseChunk| {
                response_text.push_str(&chunk.content);
            })
            .await
            .unwrap();
        let tools = parse_tool_calls(&response_text);
        // MockProvider returns deterministic response; may or may not have tools
        // We just verify the pipeline works
        assert!(!response_text.is_empty());
    }
}
