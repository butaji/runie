use super::{McpHttpClient, MCP_SESSION_HEADER};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpHttpStatus {
    Ready,
    Busy,
    Failed,
    Closed,
}

mcp_status_wire_names!(McpHttpStatus => {
    Ready => "ready",
    Busy => "busy",
    Failed => "failed",
    Closed => "closed",
});

enum McpHttpCommand {
    Request {
        request: serde_json::Value,
        reply: tokio::sync::oneshot::Sender<Result<serde_json::Value, String>>,
    },
    Stream {
        request: serde_json::Value,
        reply: tokio::sync::oneshot::Sender<Result<Vec<super::McpStreamEvent>, String>>,
    },
    Close {
        reply: tokio::sync::oneshot::Sender<Result<(), String>>,
    },
    Reconnect {
        reply: tokio::sync::oneshot::Sender<()>,
    },
}

#[derive(Clone)]
pub struct McpHttpActor {
    tx: tokio::sync::mpsc::Sender<McpHttpCommand>,
    status: tokio::sync::watch::Receiver<McpHttpStatus>,
    identity: String,
    notifications: crate::tools::McpNotificationActor,
    _owner: std::sync::Arc<crate::task_owner::TaskOwner>,
}

impl McpHttpActor {
    pub fn new(client: McpHttpClient) -> Self {
        Self::new_with_notifications(
            client,
            crate::tools::McpNotificationActor::new(crate::tools::MCP_NOTIFICATION_QUEUE_CAPACITY),
        )
    }

    pub fn new_with_notifications(
        client: McpHttpClient,
        notifications: crate::tools::McpNotificationActor,
    ) -> Self {
        let identity = client.endpoint.clone();
        let notifications_for_worker = notifications.clone();
        let (status_tx, status) = tokio::sync::watch::channel(McpHttpStatus::Ready);
        let (tx, owner) = crate::spawn_actor_worker!(32, move |rx| async move {
            run_http_worker(rx, client, notifications_for_worker, status_tx).await;
        });
        Self {
            tx,
            status,
            identity,
            notifications,
            _owner: owner,
        }
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }

    pub fn status(&self) -> McpHttpStatus {
        *self.status.borrow()
    }

    pub fn subscribe_status(&self) -> tokio::sync::watch::Receiver<McpHttpStatus> {
        self.status.clone()
    }

    pub async fn ingest_stream_events(&self, events: Vec<super::McpStreamEvent>) {
        self.notifications.ingest_stream_events(events).await;
    }

    pub async fn request(&self, request: serde_json::Value) -> Result<serde_json::Value, String> {
        let (reply, response) = tokio::sync::oneshot::channel();
        self.tx
            .send(McpHttpCommand::Request { request, reply })
            .await
            .map_err(|_| "MCP HTTP actor is closed".to_owned())?;
        response
            .await
            .map_err(|_| "MCP HTTP actor response was dropped".to_owned())?
    }

    pub async fn stream_events(
        &self,
        request: serde_json::Value,
    ) -> Result<Vec<super::McpStreamEvent>, String> {
        let (reply, response) = tokio::sync::oneshot::channel();
        self.tx
            .send(McpHttpCommand::Stream { request, reply })
            .await
            .map_err(|_| "MCP HTTP actor is closed".to_owned())?;
        response
            .await
            .map_err(|_| "MCP HTTP actor stream response was dropped".to_owned())?
    }

    pub async fn close(self) -> Result<(), String> {
        let (reply, response) = tokio::sync::oneshot::channel();
        self.tx
            .send(McpHttpCommand::Close { reply })
            .await
            .map_err(|_| "MCP HTTP actor is closed".to_owned())?;
        response
            .await
            .map_err(|_| "MCP HTTP actor close response was dropped".to_owned())?
    }

    pub async fn reconnect(&self) -> Result<(), String> {
        let (reply, response) = tokio::sync::oneshot::channel();
        self.tx
            .send(McpHttpCommand::Reconnect { reply })
            .await
            .map_err(|_| "MCP HTTP actor is closed".to_owned())?;
        response
            .await
            .map_err(|_| "MCP HTTP actor reconnect response was dropped".to_owned())
    }
}

async fn run_http_worker(
    mut rx: tokio::sync::mpsc::Receiver<McpHttpCommand>,
    client: McpHttpClient,
    notifications: crate::tools::McpNotificationActor,
    status_tx: tokio::sync::watch::Sender<McpHttpStatus>,
) {
    let mut session = McpHttpSession::new(client);
    while let Some(command) = rx.recv().await {
        if handle_http_command(&mut session, command, &notifications, &status_tx).await {
            break;
        }
    }
}

async fn handle_http_command(
    session: &mut McpHttpSession,
    command: McpHttpCommand,
    notifications: &crate::tools::McpNotificationActor,
    status_tx: &tokio::sync::watch::Sender<McpHttpStatus>,
) -> bool {
    match command {
        McpHttpCommand::Request { request, reply } => {
            let _ = status_tx.send(McpHttpStatus::Busy);
            let result = session.request(request).await;
            let _ = status_tx.send(if result.is_ok() {
                McpHttpStatus::Ready
            } else {
                McpHttpStatus::Failed
            });
            let _ = reply.send(result);
        }
        McpHttpCommand::Stream { request, reply } => {
            let _ = status_tx.send(McpHttpStatus::Busy);
            let result = session.stream_events(request).await;
            if let Ok(events) = &result {
                notifications.ingest_stream_events(events.clone()).await;
            }
            let _ = status_tx.send(if result.is_ok() {
                McpHttpStatus::Ready
            } else {
                McpHttpStatus::Failed
            });
            let _ = reply.send(result);
        }
        McpHttpCommand::Close { reply } => {
            let old = std::mem::replace(session, McpHttpSession::new(session.client.clone()));
            let _ = reply.send(old.close().await);
            let _ = status_tx.send(McpHttpStatus::Closed);
            return true;
        }
        McpHttpCommand::Reconnect { reply } => {
            *session = McpHttpSession::new(session.client.clone());
            let _ = status_tx.send(McpHttpStatus::Ready);
            let _ = reply.send(());
        }
    }
    false
}

/// Stateful streamable-HTTP MCP session. Session ownership is explicit and
/// closure is awaited by the caller, so cleanup is not detached.
pub struct McpHttpSession {
    pub(crate) client: McpHttpClient,
    session_id: Option<String>,
}

impl McpHttpSession {
    pub fn new(client: McpHttpClient) -> Self {
        Self {
            client,
            session_id: None,
        }
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub async fn request(
        &mut self,
        request: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let (value, session_id) = self
            .client
            .request_with_session(request, self.session_id.as_deref())
            .await?;
        if session_id.is_some() {
            self.session_id = session_id;
        }
        Ok(value)
    }

    pub(crate) async fn stream_events(
        &mut self,
        request: serde_json::Value,
    ) -> Result<Vec<super::McpStreamEvent>, String> {
        let (events, session_id) = self
            .client
            .stream_events_with_session(request, self.session_id.as_deref())
            .await?;
        if session_id.is_some() {
            self.session_id = session_id;
        }
        Ok(events)
    }

    pub async fn close(self) -> Result<(), String> {
        let Some(session_id) = self.session_id else {
            return Ok(());
        };
        let client = reqwest::Client::builder()
            .timeout(self.client.timeout)
            .build()
            .map_err(|error| error.to_string())?;
        let mut request = client
            .delete(&self.client.endpoint)
            .header(MCP_SESSION_HEADER, session_id);
        if let Some(token) = &self.client.bearer_token {
            request = request.bearer_auth(token);
        }
        request
            .send()
            .await
            .map_err(|error| format!("MCP session close: {error}"))?
            .error_for_status()
            .map(|_| ())
            .map_err(|error| format!("MCP session close: {error}"))
    }
}
