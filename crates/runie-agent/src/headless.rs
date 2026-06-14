//! Shared headless runner for non-interactive binaries.
//!
//! `run_headless_turn` streams a single turn from a provider, optionally
//! executes any parsed tool calls, and continues the conversation for up to
//! `max_tool_rounds` rounds. The server mode sets `execute_tools: false` and
//! simply returns the streamed content.

use crate::parser::parse_tool_calls;
use crate::Tool;
use anyhow::Result;
use futures::StreamExt;
use runie_core::provider::{Message, Provider};
use serde_json::Value;

/// Result of a headless turn.
#[derive(Debug, Clone)]
pub struct HeadlessResult {
    /// All streamed assistant text accumulated across tool rounds.
    pub content: String,
    /// Tool calls that were executed (only populated when `execute_tools` is true).
    pub tool_outputs: Vec<ToolOutput>,
}

/// A captured tool execution.
#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub name: String,
    pub arguments: Value,
    pub output: String,
}

/// Options controlling `run_headless_turn`.
pub struct HeadlessOptions<'a> {
    /// Run parsed tools and append their results for another model turn.
    pub execute_tools: bool,
    /// Maximum number of tool/model round trips.
    pub max_tool_rounds: usize,
    /// Optional per-chunk streaming callback.
    pub on_chunk: Option<&'a mut (dyn FnMut(&str) + Send)>,
    /// Required when `execute_tools` is true; serializes a tool's arguments.
    pub tool_to_json: Option<&'a (dyn Fn(&Tool) -> Value + Send + Sync)>,
}

impl<'a> HeadlessOptions<'a> {
    /// Options for a simple one-shot stream with no tool execution.
    pub fn stream_only() -> Self {
        Self {
            execute_tools: false,
            max_tool_rounds: 0,
            on_chunk: None,
            tool_to_json: None,
        }
    }
}

/// Stream one turn from `provider`, optionally executing tools.
///
/// The caller must already include the system and user messages in `messages`.
/// The helper does not modify the initial messages except to append assistant
/// responses and tool results during tool rounds.
pub async fn run_headless_turn(
    messages: Vec<Message>,
    provider: &dyn Provider,
    mut options: HeadlessOptions<'_>,
) -> Result<HeadlessResult> {
    let mut messages = messages;
    let mut content = String::new();
    let mut tool_outputs = Vec::new();

    for _ in 0..options.max_tool_rounds.max(1) {
        let mut response_text = String::new();
        let mut stream = provider.generate(messages.clone());
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            response_text.push_str(&chunk.content);
            content.push_str(&chunk.content);
            if let Some(cb) = options.on_chunk.as_mut() {
                cb(&chunk.content);
            }
        }

        let tools = parse_tool_calls(&response_text);
        if tools.is_empty() || !options.execute_tools {
            break;
        }

        messages.push(Message::Assistant {
            content: response_text,
        });
        execute_headless_tools(&tools, &mut messages, &mut tool_outputs, &options)?;
    }

    Ok(HeadlessResult {
        content,
        tool_outputs,
    })
}

fn execute_headless_tools(
    tools: &[Tool],
    messages: &mut Vec<Message>,
    tool_outputs: &mut Vec<ToolOutput>,
    options: &HeadlessOptions<'_>,
) -> Result<()> {
    let tool_to_json = options
        .tool_to_json
        .ok_or_else(|| anyhow::anyhow!("tool_to_json is required when execute_tools is true"))?;

    for tool in tools {
        let result = tool.execute();
        tool_outputs.push(ToolOutput {
            name: tool.name().to_string(),
            arguments: tool_to_json(tool),
            output: result.output.clone(),
        });
        messages.push(Message::ToolResult {
            content: format!("{} result:\n{}", tool.name(), result.output),
        });
    }
    Ok(())
}

fn read_file_json(path: &str, offset: Option<usize>, limit: Option<usize>) -> Value {
    let mut m = serde_json::Map::new();
    m.insert("path".into(), path.into());
    if let Some(o) = offset {
        m.insert("offset".into(), o.into());
    }
    if let Some(l) = limit {
        m.insert("limit".into(), l.into());
    }
    Value::Object(m)
}

fn grep_json(
    pattern: &str,
    path: &str,
    glob: &Option<String>,
    ignore_case: bool,
    literal: bool,
    context: usize,
    limit: usize,
) -> Value {
    let mut m = serde_json::Map::new();
    m.insert("pattern".into(), pattern.into());
    m.insert("path".into(), path.into());
    if let Some(g) = glob {
        m.insert("glob".into(), g.clone().into());
    }
    m.insert("ignore_case".into(), ignore_case.into());
    m.insert("literal".into(), literal.into());
    m.insert("context".into(), context.into());
    m.insert("limit".into(), limit.into());
    Value::Object(m)
}

fn file_tool_to_json(tool: &Tool) -> Option<Value> {
    Some(match tool {
        Tool::ReadFile {
            path,
            offset,
            limit,
        } => read_file_json(path, *offset, *limit),
        Tool::ListDir { path } => serde_json::json!({"path": path}),
        Tool::WriteFile { path, content } => {
            serde_json::json!({"path": path, "content": content})
        }
        Tool::EditFile {
            path,
            search,
            replace,
        } => serde_json::json!({"path": path, "search": search, "replace": replace}),
        _ => return None,
    })
}

/// Serialize a [`Tool`] into its JSON argument representation.
pub fn tool_to_json(tool: &Tool) -> Value {
    if let Some(json) = file_tool_to_json(tool) {
        return json;
    }
    match tool {
        Tool::Bash { command } => serde_json::json!({"command": command}),
        Tool::Grep {
            pattern,
            path,
            glob,
            ignore_case,
            literal,
            context,
            limit,
        } => grep_json(
            pattern,
            path,
            glob,
            *ignore_case,
            *literal,
            *context,
            *limit,
        ),
        Tool::Find {
            pattern,
            path,
            limit,
        } => serde_json::json!({"pattern": pattern, "path": path, "limit": limit}),
        Tool::FetchDocs { library } => serde_json::json!({"library": library}),
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::ensure_mock_provider;
    use runie_provider::MockProvider;

    #[tokio::test]
    async fn headless_runner_with_mock_returns_content() {
        let provider = MockProvider::default();
        let messages = vec![
            Message::System {
                content: "You are helpful.".into(),
            },
            Message::User {
                content: "hello world".into(),
            },
        ];
        let mut chunks = Vec::new();
        let options = HeadlessOptions {
            execute_tools: false,
            max_tool_rounds: 5,
            on_chunk: Some(&mut |c: &str| chunks.push(c.to_string())),
            tool_to_json: None,
        };
        let result = run_headless_turn(messages, &provider, options)
            .await
            .unwrap();
        assert!(!result.content.is_empty());
        assert_eq!(result.content, chunks.join(""));
        assert!(result.tool_outputs.is_empty());
    }

    #[tokio::test]
    async fn headless_runner_executes_tool_and_returns_output() {
        ensure_mock_provider();
        let provider = MockProvider::default();
        let messages = vec![
            Message::System {
                content: "You are helpful.".into(),
            },
            Message::User {
                content: "list files".into(),
            },
        ];
        let options = HeadlessOptions {
            execute_tools: true,
            max_tool_rounds: 5,
            on_chunk: None,
            tool_to_json: Some(&tool_to_json),
        };
        let result = run_headless_turn(messages, &provider, options)
            .await
            .unwrap();
        assert!(!result.content.is_empty());
        assert_eq!(result.tool_outputs.len(), 1);
        assert_eq!(result.tool_outputs[0].name, "list_dir");
        assert!(result.tool_outputs[0].arguments.get("path").is_some());
        assert!(!result.tool_outputs[0].output.is_empty());
    }
}
