//! runie-server — JSON-RPC-ish server for IDE integration using `runie-protocol`.
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
use runie_agent::{run_headless_turn, HeadlessOptions};
use runie_core::{config_reload, message::ChatMessage};
use runie_protocol::{Error, Message, Request, Response};
use serde_json::Value;
use std::collections::HashMap;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

const CURRENT_VERSION: &str = runie_protocol::PROTOCOL_VERSION;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let use_stdio = args.iter().any(|a| a == "--stdio");

    let result = if use_stdio {
        run_stdio_server().await
    } else {
        run_tcp_server().await
    };

    if let Err(e) = result {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
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
            _ = &mut shutdown => break,
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
        write_response(&mut stdout, &process_request(&line).await).await?;
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
        let _ = write_response(&mut writer, &process_request(&line).await).await;
    }
}

async fn write_response<W>(writer: &mut W, msg: &Message) -> Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    let json = serde_json::to_string(msg)?;
    writer.write_all(json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}

async fn process_request(line: &str) -> Message {
    let req = match serde_json::from_str::<Request>(line) {
        Ok(r) => r,
        Err(e) => return Message::error(None, Error::parse(format!("Parse error: {e}"))),
    };

    let id = req.id.clone();
    match dispatch_method(&req).await {
        Ok(result) => Message::Response(Response::ok(id, result.unwrap_or(Value::Null))),
        Err(e) => Message::Response(Response::err(id, e)),
    }
}

async fn dispatch_method(req: &Request) -> Result<Option<Value>, Error> {
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
        _ => Err(Error::method_not_found(format!(
            "Method not found: {}",
            req.method
        ))),
    }
}

fn initialize_result() -> Value {
    serde_json::json!({ "name": "runie-server", "version": env!("CARGO_PKG_VERSION"), "protocolVersion": CURRENT_VERSION })
}

fn chat_error(e: anyhow::Error) -> Error {
    Error::internal(format!("Chat error: {e}"))
}

fn complete_error(e: anyhow::Error) -> Error {
    Error::internal(format!("Complete error: {e}"))
}

fn list_sessions_error(e: anyhow::Error) -> Error {
    Error::internal(format!("List sessions error: {e}"))
}

fn load_config() -> runie_core::config::Config {
    config_reload::Config::load(Some(&config_reload::config_path()))
}

fn build_headless_provider(
    config: &runie_core::config::Config,
) -> Result<runie_provider::DynProvider, Error> {
    let chain = config.provider_chain();
    let model = config.default_model().unwrap_or("echo");
    runie_provider::build_provider_with_fallback(&chain, model)
        .map_err(|e| Error::internal(format!("{e}")))
}

fn headless_system_prompt() -> String {
    runie_core::prompts::build_system_prompt(
        runie_core::prompts::DEFAULT_PROMPT,
        runie_core::prompts::DEFAULT_TOOLS,
        false,
        "",
    )
}

fn headless_options() -> HeadlessOptions {
    HeadlessOptions {
        execute_tools: false,
        max_tool_rounds: 1,
        on_chunk: None,
    }
}

async fn handle_chat(params: &Value) -> Result<Value> {
    let messages: Vec<ChatMessage> =
        serde_json::from_value(params.get("messages").cloned().unwrap_or_default())?;
    let config = load_config();
    let provider = build_headless_provider(&config)?;

    let mut msgs = vec![ChatMessage::system(headless_system_prompt())];
    msgs.extend(messages);

    let result = run_headless_turn(msgs, &provider, headless_options()).await?;
    Ok(serde_json::json!({ "content": result.content }))
}

async fn handle_complete(params: &Value) -> Result<Value> {
    let prompt = params.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    let config = load_config();
    let provider = build_headless_provider(&config)?;

    let msgs = vec![
        ChatMessage::system(headless_system_prompt()),
        ChatMessage::user(prompt.to_string()),
    ];

    let result = run_headless_turn(msgs, &provider, headless_options()).await?;
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
    use runie_protocol::{Message, Version};

    #[test]
    fn rpc_parses_request() {
        let json =
            r#"{"kind":"request","id":1,"method":"initialize","params":{},"version":"0.1.0"}"#;
        let msg: Message = serde_json::from_str(json).unwrap();
        let Message::Request(req) = msg else {
            panic!("expected request")
        };
        assert_eq!(req.method, "initialize");
        assert_eq!(req.version, Version::current());
    }

    #[test]
    fn rpc_returns_response() {
        let msg = Message::response(Some(1.into()), serde_json::json!({ "ok": true }));
        let s = serde_json::to_string(&msg).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed["kind"], "response");
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
