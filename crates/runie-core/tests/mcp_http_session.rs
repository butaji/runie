use runie_core::tools::{McpHttpClient, McpHttpSession};
use std::time::Duration;

#[tokio::test]
async fn http_transport_posts_json_and_decodes_response() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut tasks = tokio::task::JoinSet::new();
    tasks.spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0_u8; 2048];
        let _ = tokio::io::AsyncReadExt::read(&mut socket, &mut request).await;
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
            body.len(),
            body
        );
        tokio::io::AsyncWriteExt::write_all(&mut socket, response.as_bytes())
            .await
            .unwrap();
    });
    let client = McpHttpClient::new(
        format!("http://{address}"),
        Some("secret".into()),
        Duration::from_secs(1),
    )
    .unwrap();
    let response = client
        .request(serde_json::json!({"jsonrpc":"2.0","id":1}))
        .await
        .unwrap();
    tasks.join_next().await.unwrap().unwrap();
    assert_eq!(response["result"]["ok"], true);
}

#[tokio::test]
async fn http_session_reuses_and_closes_server_session() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut tasks = tokio::task::JoinSet::new();
    tasks.spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0_u8; 2048];
        let size = tokio::io::AsyncReadExt::read(&mut socket, &mut request)
            .await
            .unwrap();
        assert!(String::from_utf8_lossy(&request[..size]).starts_with("POST"));
        let body = r#"{"jsonrpc":"2.0","id":1,"result":{"ok":true}}"#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nMcp-Session-Id: session-1\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
            body.len(), body
        );
        tokio::io::AsyncWriteExt::write_all(&mut socket, response.as_bytes())
            .await
            .unwrap();
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0_u8; 2048];
        let size = tokio::io::AsyncReadExt::read(&mut socket, &mut request)
            .await
            .unwrap();
        let request = String::from_utf8_lossy(&request[..size]).to_ascii_lowercase();
        assert!(request.starts_with("delete"));
        assert!(request.contains("mcp-session-id: session-1"));
        tokio::io::AsyncWriteExt::write_all(
            &mut socket,
            b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n",
        )
        .await
        .unwrap();
    });
    let client =
        McpHttpClient::new(format!("http://{address}"), None, Duration::from_secs(1)).unwrap();
    let mut session = McpHttpSession::new(client);
    assert_eq!(
        session.request(serde_json::json!({"id":1})).await.unwrap()["result"]["ok"],
        true
    );
    assert_eq!(session.session_id(), Some("session-1"));
    session.close().await.unwrap();
    tasks.join_next().await.unwrap().unwrap();
}
