//! runie-server — JSON-RPC 2.0 server for IDE integration.
//!
//! ## Protocol
//! Transport: TCP (port printed on startup) or stdio.
//! Each message is a JSON object terminated by a newline.
//!
//! ## Methods
//! - `initialize` → `{}`
//! - `chat` → `{ "messages": [{"role":"user","content":"hi"}] }` → `{ "content": "..." }`
//! - `complete` → `{ "prompt": "..." }` → `{ "content": "..." }`
//! - `listModels` → `{}` → `{ "models": [...] }`
//! - `listSessions` → `{}` → `{ "sessions": [...] }`

use anyhow::Result;
use runie_agent::{build_provider_with_warning, run_headless_turn, HeadlessOptions};
use runie_core::{config_reload, message::ChatMessage, provider::Message};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

/// JSON-RPC 2.0 request.
#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

/// JSON-RPC 2.0 response.
#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_stdio = args.iter().any(|a| a == "--stdio");

    if use_stdio {
        if let Err(e) = run_stdio_server().await {
            eprintln!("Server error: {}", e);
            std::process::exit(1);
        }
    } else {
        if let Err(e) = run_tcp_server().await {
            eprintln!("Server error: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run_tcp_server() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    println!("{}", port);

    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    loop {
        tokio::select! {
            Ok((stream, _)) = listener.accept() => {
                tokio::spawn(handle_connection(stream));
            }
            _ = &mut shutdown => {
                break;
            }
        }
    }

    Ok(())
}

async fn run_stdio_server() -> Result<()> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();
    let mut stdout = stdout;

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        let response = process_request(&line).await;
        let json = serde_json::to_string(&response)?;
        stdout.write_all(json.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }

    Ok(())
}

async fn handle_connection(stream: TcpStream) {
    let (read_half, write_half) = stream.into_split();
    let reader = BufReader::new(read_half);
    let mut lines = reader.lines();
    let mut writer = write_half;

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().is_empty() {
            continue;
        }
        let response = process_request(&line).await;
        if let Ok(json) = serde_json::to_string(&response) {
            let _ = writer.write_all(json.as_bytes()).await;
            let _ = writer.write_all(b"\n").await;
            let _ = writer.flush().await;
        }
    }
}

async fn process_request(line: &str) -> JsonRpcResponse {
    let req = match serde_json::from_str::<JsonRpcRequest>(line) {
        Ok(r) => r,
        Err(e) => return parse_error_response(e),
    };

    let result = match dispatch_method(&req).await {
        Ok(r) => r,
        Err((code, msg)) => return error_response(req.id, code, msg),
    };

    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id: req.id,
        result,
        error: None,
    }
}

fn parse_error_response(e: serde_json::Error) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id: None,
        result: None,
        error: Some(JsonRpcError {
            code: -32700,
            message: format!("Parse error: {}", e),
            data: None,
        }),
    }
}

async fn dispatch_method(req: &JsonRpcRequest) -> Result<Option<Value>, (i32, String)> {
    match req.method.as_str() {
        "initialize" => Ok(Some(initialize_result())),
        "chat" => handle_chat(&req.params).await.map(Some).map_err(chat_error),
        "complete" => handle_complete(&req.params)
            .await
            .map(Some)
            .map_err(complete_error),
        "listModels" => Ok(Some(handle_list_models())),
        "listSessions" => handle_list_sessions()
            .map(Some)
            .map_err(list_sessions_error),
        _ => Err((-32601, format!("Method not found: {}", req.method))),
    }
}

fn initialize_result() -> Value {
    serde_json::json!({ "name": "runie-server", "version": env!("CARGO_PKG_VERSION") })
}

fn chat_error(e: anyhow::Error) -> (i32, String) {
    (-32603, format!("Chat error: {}", e))
}

fn complete_error(e: anyhow::Error) -> (i32, String) {
    (-32603, format!("Complete error: {}", e))
}

fn list_sessions_error(e: anyhow::Error) -> (i32, String) {
    (-32603, format!("List sessions error: {}", e))
}

fn error_response(id: Option<Value>, code: i32, message: String) -> JsonRpcResponse {
    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: None,
        error: Some(JsonRpcError {
            code,
            message,
            data: None,
        }),
    }
}

async fn handle_chat(params: &Value) -> Result<Value> {
    let messages: Vec<ChatMessage> =
        serde_json::from_value(params.get("messages").cloned().unwrap_or_default())?;
    let config = config_reload::Config::load_from(&config_reload::config_path());
    let provider_name = config.provider.as_deref().unwrap_or("mock");
    let model = config.default_model().unwrap_or("echo");
    let provider =
        build_provider_with_warning(provider_name, model).map_err(|e| anyhow::anyhow!("{}", e))?;

    let system = runie_core::prompts::build_system_prompt(
        runie_core::prompts::DEFAULT_PROMPT,
        runie_core::prompts::DEFAULT_TOOLS,
        false,
        "",
    );

    let mut msgs = vec![Message::System { content: system }];
    for m in &messages {
        msgs.push(m.to_provider_message());
    }

    let options = HeadlessOptions {
        execute_tools: false,
        max_tool_rounds: 1,
        on_chunk: None,
    };
    let result = run_headless_turn(msgs, &provider, options).await?;

    Ok(serde_json::json!({ "content": result.content }))
}

async fn handle_complete(params: &Value) -> Result<Value> {
    let prompt = params.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    let config = config_reload::Config::load_from(&config_reload::config_path());
    let provider_name = config.provider.as_deref().unwrap_or("mock");
    let model = config.default_model().unwrap_or("echo");
    let provider =
        build_provider_with_warning(provider_name, model).map_err(|e| anyhow::anyhow!("{}", e))?;

    let system = runie_core::prompts::build_system_prompt(
        runie_core::prompts::DEFAULT_PROMPT,
        runie_core::prompts::DEFAULT_TOOLS,
        false,
        "",
    );

    let msgs = vec![
        Message::System { content: system },
        Message::User {
            content: prompt.to_string(),
        },
    ];

    let options = HeadlessOptions {
        execute_tools: false,
        max_tool_rounds: 1,
        on_chunk: None,
    };
    let result = run_headless_turn(msgs, &provider, options).await?;

    Ok(serde_json::json!({ "content": result.content }))
}

fn handle_list_models() -> Value {
    let catalog = runie_core::model_catalog::model_catalog();
    let models: Vec<HashMap<String, String>> = catalog
        .iter()
        .map(|m| {
            let mut map = HashMap::new();
            map.insert("name".into(), m.name.clone());
            map.insert("provider".into(), m.provider.clone());
            map.insert("displayName".into(), m.display_name.clone());
            map
        })
        .collect();
    serde_json::json!({ "models": models })
}

fn handle_list_sessions() -> Result<Value> {
    let sessions = runie_core::session::list()?;
    Ok(serde_json::json!({ "sessions": sessions }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rpc_parses_request() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "initialize");
    }

    #[test]
    fn rpc_returns_response() {
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: Some(1.into()),
            result: Some(serde_json::json!({ "ok": true })),
            error: None,
        };
        let s = serde_json::to_string(&resp).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["id"], 1);
        assert_eq!(parsed["result"]["ok"], true);
        assert!(parsed.get("error").is_none() || parsed["error"].is_null());
    }

    #[test]
    fn rpc_list_models() {
        let result = handle_list_models();
        let models = result.get("models").unwrap().as_array().unwrap();
        assert!(!models.is_empty(), "Model catalog should not be empty");
    }
}
