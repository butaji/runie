//! Agent Communication Protocol (ACP) for Runie.
//!
//! Provides Unix domain socket communication between the Runie leader process
//! and clients (TUI, CLI, IDE extensions).
//!
//! ## Architecture
//!
//! - Single leader per machine
//! - Clients connect via Unix domain sockets at `~/.runie/leader.sock`
//! - Message framing: 4-byte big-endian length prefix + JSON body
//! - Maximum message size: 64 MiB

pub mod message;
pub mod protocol;
pub mod server;
pub mod client;

pub use message::{ClientMessage, ServerMessage, AcpMessage, ClientMode, ClientCapabilities};
pub use protocol::AcpProtocol;
pub use server::LeaderServer;
pub use client::ClientConnection;

use anyhow::Result;
use std::path::PathBuf;

/// Default socket path for the leader.
pub fn default_socket_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".runie").join("leader.sock")
}

/// Maximum message size (64 MiB).
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024 * 1024;

/// Initialize the ACP subsystem.
///
/// Creates the socket directory if it doesn't exist.
pub fn init() -> Result<()> {
    let socket_path = default_socket_path();
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}
