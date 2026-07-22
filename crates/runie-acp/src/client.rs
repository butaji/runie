//! ACP client connection.
//!
//! Handles client-side connection to the leader server.

use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::net::UnixStream;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use super::{message::ClientMessage, message::ServerMessage, ClientMode, ClientCapabilities, AcpProtocol};

/// Client connection handle.
pub struct ClientConnection {
    write_half: Arc<Mutex<tokio::io::WriteHalf<UnixStream>>>, // Tokio Mutex for async-compatible locking
    #[allow(dead_code)]
    client_id: Option<u64>,
}

impl std::fmt::Debug for ClientConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientConnection")
            .field("client_id", &self.client_id)
            .finish()
    }
}

impl ClientConnection {
    /// Connect to the leader server.
    /// Returns the connection and a receiver for incoming server messages.
    pub async fn connect(path: &Path) -> Result<(Self, mpsc::Receiver<ServerMessage>)> {
        info!("Connecting to leader at {:?}", path);
        let stream = UnixStream::connect(path).await.context("failed to connect to leader")?;

        let (read_half, write_half) = tokio::io::split(stream);
        let (msg_tx, msg_rx) = mpsc::channel(100);

        // Move read_half into background task
        tokio::spawn(async move {
            Self::read_loop(read_half, msg_tx).await;
        });

        let conn = Self {
            write_half: Arc::new(Mutex::new(write_half)),
            client_id: None,
        };

        Ok((conn, msg_rx))
    }

    /// Read loop for processing server messages.
    async fn read_loop<R: tokio::io::AsyncRead + Unpin>(
        mut reader: R,
        tx: mpsc::Sender<ServerMessage>,
    ) {
        loop {
            match AcpProtocol::read_message::<_, ServerMessage>(&mut reader).await {
                Ok(msg) => {
                    debug!("Received: {:?}", msg);
                    if tx.send(msg).await.is_err() {
                        debug!("receiver dropped, stopping read loop");
                        break;
                    }
                }
                Err(e) => {
                    error!("read error: {}", e);
                    break;
                }
            }
        }
    }

    /// Register this client with the leader and wait for the response via rx.
    pub async fn register(
        &self,
        client_type: &str,
        mode: ClientMode,
        capabilities: ClientCapabilities,
        mut rx: mpsc::Receiver<ServerMessage>,
    ) -> Result<u64> {
        let msg = ClientMessage::Register {
            client_type: client_type.to_string(),
            mode,
            capabilities,
        };

        {
            let mut write = self.write_half.lock().await;
            AcpProtocol::write_message(&mut *write, &msg).await?;
        }

        let response = rx.recv().await.ok_or_else(|| {
            anyhow::anyhow!("connection closed before receiving registration response")
        })?;

        match response {
            ServerMessage::Registered { client_id, .. } => Ok(client_id),
            ServerMessage::Error { message, .. } => {
                anyhow::bail!("registration failed: {}", message);
            }
            _ => anyhow::bail!("unexpected message during registration: {:?}", response),
        }
    }

    /// Send a message to the leader.
    pub async fn send(&self, msg: ClientMessage) -> Result<()> {
        let mut write = self.write_half.lock().await;
        AcpProtocol::write_message(&mut *write, &msg).await
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.client_id.is_some()
    }
}
