//! ACP (Agent Protocol) implementation for runie CLI.
//!
//! Runs as an ACP agent over JSON-RPC on stdin/stdout.

use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};
use tokio::sync::Mutex;
use std::sync::Arc;

/// JSON-RPC request structure
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

/// JSON-RPC response structure
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcResponse {
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: serde_json::Value, code: i32, message: &str) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data: None,
            }),
        }
    }
}

/// ACP session state
pub struct AcpSession {
    pub session_id: Option<String>,
    pub cwd: Option<String>,
    pub authenticated: bool,
    pub api_key: Option<String>,
}

impl Default for AcpSession {
    fn default() -> Self {
        Self {
            session_id: None,
            cwd: None,
            authenticated: false,
            api_key: None,
        }
    }
}

/// Shared ACP state
pub struct AcpState {
    pub session: Mutex<AcpSession>,
}

impl Default for AcpState {
    fn default() -> Self {
        Self {
            session: Mutex::new(AcpSession::default()),
        }
    }
}

/// Run the ACP stdio loop
pub async fn run_acp_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let state = Arc::new(AcpState::default());
    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin);

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(req) => req,
            Err(e) => {
                let response = JsonRpcResponse::error(
                    serde_json::Value::Null,
                    -32700,
                    &format!("Parse error: {}", e),
                );
                print_response(&response);
                continue;
            }
        };

        let response = handle_request(&state, request).await;
        print_response(&response);
    }

    Ok(())
}

fn print_response(response: &JsonRpcResponse) {
    if let Ok(json) = serde_json::to_string(response) {
        println!("{}", json);
    }
}

async fn handle_request(
    state: &Arc<AcpState>,
    request: JsonRpcRequest,
) -> JsonRpcResponse {
    // Verify JSON-RPC version
    if request.jsonrpc != "2.0" {
        return JsonRpcResponse::error(
            request.id,
            -32600,
            "Invalid Request: jsonrpc must be '2.0'",
        );
    }

    match request.method.as_str() {
        "initialize" => handle_initialize(state, request.id, request.params).await,
        "authenticate" => handle_authenticate(state, request.id, request.params).await,
        "session/new" => handle_session_new(state, request.id, request.params).await,
        "session/prompt" => handle_session_prompt(state, request.id, request.params).await,
        _ => JsonRpcResponse::error(
            request.id,
            -32601,
            &format!("Method not found: {}", request.method),
        ),
    }
}

async fn handle_initialize(
    state: &Arc<AcpState>,
    id: serde_json::Value,
    _params: serde_json::Value,
) -> JsonRpcResponse {
    #[derive(Serialize)]
    struct InitializeResult {
        protocol_version: String,
        capabilities: Capabilities,
        server_info: ServerInfo,
    }

    #[derive(Serialize)]
    struct Capabilities {
        tools: bool,
        sessions: bool,
        authentication: bool,
    }

    #[derive(Serialize)]
    struct ServerInfo {
        name: String,
        version: String,
    }

    let result = InitializeResult {
        protocol_version: "1.0".to_string(),
        capabilities: Capabilities {
            tools: true,
            sessions: true,
            authentication: true,
        },
        server_info: ServerInfo {
            name: "runie".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    };

    JsonRpcResponse::success(id, serde_json::to_value(result).unwrap_or_default())
}

async fn handle_authenticate(
    state: &Arc<AcpState>,
    id: serde_json::Value,
    params: serde_json::Value,
) -> JsonRpcResponse {
    #[derive(Deserialize)]
    struct AuthParams {
        method: String,
        api_key: Option<String>,
    }

    let params = match serde_json::from_value::<AuthParams>(params) {
        Ok(p) => p,
        Err(e) => {
            return JsonRpcResponse::error(id, -32602, &format!("Invalid params: {}", e));
        }
    };

    let mut session = state.session.lock().await;

    match params.method.as_str() {
        "api_key" => {
            if let Some(ref key) = params.api_key {
                session.authenticated = true;
                session.api_key = Some(key.clone());
                JsonRpcResponse::success(
                    id,
                    serde_json::json!({"status": "authenticated"}),
                )
            } else {
                JsonRpcResponse::error(id, -32602, "api_key required for api_key method")
            }
        }
        "env" => {
            // Authenticate using environment variables
            let key = std::env::var("XAI_API_KEY")
                .or_else(|_| std::env::var("OPENAI_API_KEY"))
                .or_else(|_| std::env::var("RUNIE_API_KEY"));

            match key {
                Ok(k) => {
                    session.authenticated = true;
                    session.api_key = Some(k);
                    JsonRpcResponse::success(
                        id,
                        serde_json::json!({"status": "authenticated", "method": "env"}),
                    )
                }
                Err(_) => JsonRpcResponse::error(
                    id,
                    -32603,
                    "No API key found in environment (XAI_API_KEY, OPENAI_API_KEY, or RUNIE_API_KEY)",
                ),
            }
        }
        _ => JsonRpcResponse::error(
            id,
            -32602,
            &format!("Unknown auth method: {}", params.method),
        ),
    }
}

async fn handle_session_new(
    state: &Arc<AcpState>,
    id: serde_json::Value,
    params: serde_json::Value,
) -> JsonRpcResponse {
    #[derive(Deserialize)]
    struct SessionNewParams {
        session_id: Option<String>,
        cwd: Option<String>,
    }

    let params = match serde_json::from_value::<SessionNewParams>(params) {
        Ok(p) => p,
        Err(e) => {
            return JsonRpcResponse::error(id, -32602, &format!("Invalid params: {}", e));
        }
    };

    let mut session = state.session.lock().await;
    let new_session_id = params.session_id.unwrap_or_else(|| uuid_simple());

    session.session_id = Some(new_session_id.clone());
    session.cwd = params.cwd;

    JsonRpcResponse::success(
        id,
        serde_json::json!({
            "session_id": new_session_id,
            "status": "created"
        }),
    )
}

async fn handle_session_prompt(
    state: &Arc<AcpState>,
    id: serde_json::Value,
    params: serde_json::Value,
) -> JsonRpcResponse {
    #[derive(Deserialize)]
    struct SessionPromptParams {
        session_id: Option<String>,
        prompt: String,
    }

    let params = match serde_json::from_value::<SessionPromptParams>(params) {
        Ok(p) => p,
        Err(e) => {
            return JsonRpcResponse::error(id, -32602, &format!("Invalid params: {}", e));
        }
    };

    let session = state.session.lock().await;

    // Verify session exists
    if session.session_id.is_none() {
        return JsonRpcResponse::error(
            id,
            -32603,
            "No active session. Call session/new first.",
        );
    }

    let current_session_id = session.session_id.clone().unwrap();

    // If session_id provided, verify it matches
    if let Some(ref provided) = params.session_id {
        if provided != &current_session_id {
            return JsonRpcResponse::error(
                id,
                -32603,
                &format!("Session mismatch: expected {}, got {}", current_session_id, provided),
            );
        }
    }

    drop(session);

    // Process the prompt using the provider
    let response = process_prompt(&params.prompt).await;

    match response {
        Ok(content) => JsonRpcResponse::success(
            id,
            serde_json::json!({
                "session_id": current_session_id,
                "response": content
            }),
        ),
        Err(e) => JsonRpcResponse::error(id, -32603, &e.to_string()),
    }
}

async fn process_prompt(prompt: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use crate::provider_factory::create_provider;
    use crate::settings::Settings;
    use runie_ai::providers::MockProvider;
    use runie_ai::Provider;

    let settings = Settings::load();

    // Try to create a real provider first
    let use_mock = settings.api_key.is_none()
        && std::env::var("OPENAI_API_KEY").is_err()
        && std::env::var("XAI_API_KEY").is_err()
        && std::env::var("RUNIE_API_KEY").is_err();

    if use_mock {
        let mock_provider = MockProvider::new();
        let messages = vec![runie_core::Message::User {
            content: prompt.to_string(),
            attachments: vec![],
        }];
        mock_provider.chat_simple(messages).await.map_err(|e| e.into())
    } else {
        let provider = create_provider(false, &settings)?;
        let messages = vec![runie_core::Message::User {
            content: prompt.to_string(),
            attachments: vec![],
        }];
        provider.chat_simple(messages).await.map_err(|e| e.into())
    }
}

/// Generate a simple UUID-like string (no external dependency)
fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}-{:x}", timestamp, std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_response_success() {
        let response = JsonRpcResponse::success(
            serde_json::json!(1),
            serde_json::json!({"key": "value"}),
        );
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }

    #[test]
    fn test_jsonrpc_response_error() {
        let response = JsonRpcResponse::error(serde_json::json!(1), -32601, "Method not found");
        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32601);
    }
}
