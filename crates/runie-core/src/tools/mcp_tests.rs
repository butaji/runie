use super::*;

#[test]
fn server_and_tool_are_data_with_stable_names() {
    let server = McpServer {
        name: "files".into(),
        tools: vec![McpToolSpec {
            name: "list".into(),
            description: "List files".into(),
            input_schema: empty_schema(),
        }],
    };
    assert_eq!(server.tools[0].name, "list");
}

#[test]
fn status_rows_own_their_terminal_projection() {
    let row = McpStatusRow {
        transport: "http".into(),
        index: 2,
        status: "busy".into(),
    };
    assert_eq!(row.terminal_line(), "http[2] status=busy");
    assert_eq!(
        serde_json::from_value::<McpStatusRow>(serde_json::to_value(&row).unwrap()).unwrap(),
        row
    );
}

#[tokio::test]
async fn mcp_tool_forwards_a_typed_call_to_its_owner() {
    let tool = McpTool::new(
        "files",
        McpToolSpec {
            name: "list".into(),
            description: "List files".into(),
            input_schema: empty_schema(),
        },
        Arc::new(|call| {
            Box::pin(
                async move { Ok(serde_json::json!({"tool": call.tool, "args": call.arguments})) },
            )
        }),
    )
    .unwrap();
    assert_eq!(tool.qualified_name(), "mcp__files__list");
    assert_eq!(tool.name(), "mcp__files__list");
    let result = tool
        .execute("1", serde_json::json!({"path":"."}), None, None)
        .await
        .unwrap();
    assert_eq!(result.details["tool"], "list");
    assert_eq!(result.details["args"]["path"], ".");
}

#[tokio::test]
async fn stdio_actor_has_an_explicit_awaited_close_boundary() {
    let actor = McpStdioActor::new(
        McpStdioClient::new(
            "sh",
            vec!["-c".into(), "exit 0".into()],
            Duration::from_secs(1),
        )
        .unwrap(),
    );
    assert_eq!(actor.status(), McpStdioStatus::Ready);
    actor.clone().close().await.unwrap();
    assert_eq!(actor.status(), McpStdioStatus::Closed);
}

#[tokio::test]
async fn stdio_transport_round_trips_json_and_ignores_notifications() {
    let script = "while IFS= read -r line; do case \"$line\" in *\\\"id\\\":1*) echo '{\"method\":\"notice\"}'; echo '{\"id\":1,\"result\":{\"ok\":true}}';; esac; done";
    let client = McpStdioClient::new(
        "sh",
        vec!["-c".into(), script.into()],
        Duration::from_secs(1),
    )
    .unwrap();
    let responses = client
        .request(&[serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize"})])
        .await
        .unwrap();
    assert_eq!(responses[0]["result"]["ok"], true);
}

#[tokio::test]
async fn stdio_discovery_reduces_initialize_and_tool_list_to_server_data() {
    let script = "while IFS= read -r line; do case \"$line\" in *\\\"id\\\":1*) echo '{\"id\":1,\"result\":{\"serverInfo\":{\"name\":\"demo\"}}}';; *\\\"id\\\":2*) echo '{\"id\":2,\"result\":{\"tools\":[{\"name\":\"echo\",\"description\":\"Echo\",\"inputSchema\":{\"type\":\"object\"}}]}}';; esac; done";
    let client = McpStdioClient::new(
        "sh",
        vec!["-c".into(), script.into()],
        Duration::from_secs(1),
    )
    .unwrap();
    let server = client.discover().await.unwrap();
    assert_eq!(server.name, "demo");
    assert_eq!(server.tools[0].name, "echo");
    assert_eq!(server.tools[0].input_schema["type"], "object");
}

#[tokio::test]
async fn stdio_call_reduces_tools_call_result() {
    let script = "while IFS= read -r line; do case \"$line\" in *tools/call*) echo '{\"id\":1,\"result\":{\"content\":[{\"type\":\"text\",\"text\":\"ok\"}]}}';; esac; done";
    let client = McpStdioClient::new(
        "sh",
        vec!["-c".into(), script.into()],
        Duration::from_secs(1),
    )
    .unwrap();
    let result = client
        .call_tool("echo", serde_json::json!({"value":7}))
        .await
        .unwrap();
    assert_eq!(result["content"][0]["text"], "ok");
}

#[tokio::test]
async fn stdio_call_preserves_json_rpc_error_details() {
    let script = "while IFS= read -r line; do case \"$line\" in *tools/call*) echo '{\"id\":1,\"error\":{\"code\":-32602,\"message\":\"bad arguments\"}}';; esac; done";
    let client = McpStdioClient::new(
        "sh",
        vec!["-c".into(), script.into()],
        Duration::from_secs(1),
    )
    .unwrap();
    let error = client
        .call_tool("echo", serde_json::json!({}))
        .await
        .unwrap_err();
    assert_eq!(error, "MCP error -32602: bad arguments");
}

#[tokio::test]
async fn stdio_actor_projects_failed_call_status() {
    let script = "while IFS= read -r line; do case \"$line\" in *tools/call*) echo '{\"id\":1,\"error\":{\"code\":-32602,\"message\":\"bad arguments\"}}';; esac; done";
    let actor = McpStdioActor::new(
        McpStdioClient::new(
            "sh",
            vec!["-c".into(), script.into()],
            Duration::from_secs(1),
        )
        .unwrap(),
    );
    assert!(actor
        .call_tool("echo", serde_json::json!({}))
        .await
        .is_err());
    assert_eq!(actor.status(), McpStdioStatus::Failed);
    actor.close().await.unwrap();
}

#[tokio::test]
async fn persistent_session_initializes_once_and_reuses_process() {
    let script = r#"count=0; calls=0; while IFS= read -r line; do case "$line" in *notifications/initialized*) :;; *initialize*) count=$((count+1)); echo '{"id":1,"result":{}}';; *tools/list*) echo '{"id":2,"result":{"tools":[]}}';; *tools/call*) calls=$((calls+1)); id=$((calls+2)); echo "{\"id\":$id,\"result\":{\"count\":$count}}";; esac; done"#;
    let client = McpStdioClient::new(
        "sh",
        vec!["-c".into(), script.into()],
        Duration::from_secs(1),
    )
    .unwrap();
    let mut session = McpStdioSession::connect(&client).await.unwrap();
    assert_eq!(
        session
            .call_tool("echo", serde_json::json!({}))
            .await
            .unwrap()["count"],
        1
    );
    assert_eq!(
        session
            .call_tool("echo", serde_json::json!({}))
            .await
            .unwrap()["count"],
        1
    );
    session.close().await.unwrap();
}
