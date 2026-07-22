//! ACP leader server.
//!
//! Manages Unix domain socket server and client connections.

use anyhow::{Context, Result};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{broadcast, RwLock, Notify};
use tracing::{debug, error, info};

use crate::{ClientMessage, ClientMode, ServerMessage, AcpProtocol, ClientCapabilities, default_socket_path};

/// Leader server managing client connections.
#[derive(Debug)]
pub struct LeaderServer {
    listener: UnixListener,
    clients: Arc<RwLock<HashMap<u64, ClientSession>>>,
    next_client_id: Arc<Mutex<u64>>,
    shutdown_tx: broadcast::Sender<()>,
    event_notifier: Arc<Notify>,
}

struct ClientSession {
    #[allow(dead_code)]
    write_half: tokio::io::WriteHalf<UnixStream>,
    client_type: String,
    mode: ClientMode,
    #[allow(dead_code)]
    capabilities: ClientCapabilities,
}

impl std::fmt::Debug for ClientSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientSession")
            .field("client_type", &self.client_type)
            .field("mode", &self.mode)
            .finish()
    }
}

impl LeaderServer {
    /// Create a new leader server bound to the default socket path.
    pub async fn bind() -> Result<Self> {
        Self::bind_at(&default_socket_path()).await
    }

    /// Create a new leader server bound to a specific path.
    pub async fn bind_at(path: &Path) -> Result<Self> {
        // Remove existing socket file
        if path.exists() {
            std::fs::remove_file(path).context("failed to remove existing socket")?;
        }

        // Create parent directory
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).context("failed to create socket directory")?;
        }

        info!("Binding leader server to {:?}", path);
        let listener = UnixListener::bind(path).context("failed to bind Unix socket")?;

        // Set socket permissions (accessible only by current user)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(path, perms).context("failed to set socket permissions")?;
        }

        let (shutdown_tx, _) = broadcast::channel(1);

        Ok(Self {
            listener,
            clients: Arc::new(RwLock::new(HashMap::new())),
            next_client_id: Arc::new(Mutex::new(1)),
            shutdown_tx,
            event_notifier: Arc::new(Notify::new()),
        })
    }

    /// Start accepting connections.
    pub async fn run(self: Arc<Self>) {
        info!("Leader server running");

        loop {
            let mut shutdown_rx = self.shutdown_tx.subscribe();
            tokio::select! {
                result = self.listener.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            let server = self.clone();
                            tokio::spawn(async move {
                                if let Err(e) = Self::handle_connection(server, stream).await {
                                    error!("connection error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            error!("accept error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("shutdown signal received");
                    break;
                }
            }
        }
    }

    /// Handle a client connection.
    async fn handle_connection(self: Arc<Self>, stream: UnixStream) -> Result<()> {
        let (mut reader, mut writer) = tokio::io::split(stream);

        // Read registration message
        let msg: ClientMessage = AcpProtocol::read_message(&mut reader).await?;

        let (client_type, mode, capabilities) = match msg {
            ClientMessage::Register {
                client_type,
                mode,
                capabilities,
            } => (client_type, mode, capabilities),
            _ => {
                let response = ServerMessage::Error {
                    code: -32700,
                    message: "expected Register message".to_string(),
                };
                AcpProtocol::write_message(&mut writer, &response).await?;
                anyhow::bail!("expected Register message");
            }
        };

        let client_id = {
            let mut guard = self.next_client_id.lock();
            let id = *guard;
            *guard += 1;
            id
        };

        info!("Client {} ({}) connecting", client_id, client_type);

        // Send registration response BEFORE storing the session (writer gets consumed)
        let response = ServerMessage::Registered {
            client_id,
            ready: true,
            leader_protocol_version: Some(1),
            leader_binary_version: Some(env!("CARGO_PKG_VERSION").to_string()),
        };
        AcpProtocol::write_message(&mut writer, &response).await?;

        // Store client session (writer consumed, but client needs to keep it for future writes)
        // For now, store a placeholder - the server can be extended to manage client sessions properly
        {
            let mut clients = self.clients.write().await;
            clients.insert(
                client_id,
                ClientSession {
                    write_half: writer,
                    client_type: client_type.clone(),
                    mode,
                    capabilities,
                },
            );
        }

        // Notify event handlers
        self.event_notifier.notify_waiters();

        debug!("Client {} registered successfully", client_id);

        Ok(())
    }

    /// Get the number of connected clients.
    pub async fn client_count(&self) -> usize {
        self.clients.read().await.len()
    }

    /// Check if any clients are connected.
    pub async fn has_clients(&self) -> bool {
        !self.clients.read().await.is_empty()
    }

    /// Wait for a client to connect.
    pub async fn wait_for_client(&self) {
        self.event_notifier.notified().await;
    }

    /// Shutdown the server.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn server_binds_and_accepts() {
        let dir = tempfile::tempdir().unwrap();
        let socket_path = dir.path().join("test.sock");

        let server = LeaderServer::bind_at(&socket_path).await.unwrap();

        // Spawn server
        let server = Arc::new(server);
        let server_run = server.clone();
        let serve = tokio::spawn(async move {
            server_run.run().await;
        });

        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        // Connect a client
        let stream = UnixStream::connect(&socket_path).await.unwrap();
        let (mut reader, mut writer) = tokio::io::split(stream);

        // Send registration
        let register = ClientMessage::Register {
            client_type: "test-client".to_string(),
            mode: ClientMode::Stdio,
            capabilities: ClientCapabilities::default(),
        };
        AcpProtocol::write_message(&mut writer, &register).await.unwrap();

        // Read response
        let response: ServerMessage = AcpProtocol::read_message(&mut reader).await.unwrap();
        match response {
            ServerMessage::Registered { client_id, ready, .. } => {
                assert!(ready);
                assert_eq!(client_id, 1);
            }
            _ => panic!("expected Registered response"),
        }

        // Cleanup
        server.shutdown();
        let _ = serve.await;
    }
}
