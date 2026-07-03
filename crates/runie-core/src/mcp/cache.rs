//! Config-fingerprinted tool schema cache with disk persistence.
//!
//! Cache keys are computed as SHA-256 of the canonical JSON serialization of
//! the server configuration. Schemas are stored on disk under the cache directory.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;

use crate::config::McpServer;

/// Cached tool schema with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedToolSchema {
    /// Tool name.
    pub name: String,
    /// Tool description.
    pub description: String,
    /// JSON schema for input parameters.
    pub input_schema: serde_json::Value,
}

/// Cached tools from an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedServerSchemas {
    /// Server name (for display).
    pub server_name: String,
    /// Timestamp when cached.
    pub cached_at: chrono::DateTime<chrono::Utc>,
    /// List of tools from this server.
    pub tools: Vec<CachedToolSchema>,
}

/// Schema cache with disk persistence.
pub struct SchemaCache {
    /// In-memory cache: server_name -> CachedServerSchemas
    memory: RwLock<HashMap<String, CachedServerSchemas>>,
    /// Cache directory path.
    cache_dir: PathBuf,
}

impl SchemaCache {
    /// Create a new schema cache with the given cache directory.
    pub async fn new(cache_dir: PathBuf) -> anyhow::Result<Arc<Self>> {
        // Ensure cache directory exists
        tokio::fs::create_dir_all(&cache_dir).await?;
        let cache = Arc::new(Self {
            memory: RwLock::new(HashMap::new()),
            cache_dir,
        });
        cache.load_from_disk().await?;
        Ok(cache)
    }

    /// Load cached schemas from disk into memory.
    async fn load_from_disk(&self) -> anyhow::Result<()> {
        let mut memory = self.memory.write().await;
        let mut entries = tokio::fs::read_dir(&self.cache_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json") {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(schemas) = serde_json::from_str::<CachedServerSchemas>(&content) {
                        memory.insert(schemas.server_name.clone(), schemas);
                    }
                }
            }
        }
        Ok(())
    }

    /// Compute cache key from server configuration.
    pub fn compute_cache_key(server: &McpServer) -> String {
        let canonical = serde_json::to_string(server).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Check if a cached schema is valid for the given server config.
    pub async fn get(&self, server_name: &str, server: &McpServer) -> Option<CachedServerSchemas> {
        let memory = self.memory.read().await;
        let cached = memory.get(server_name)?;
        let expected_key = Self::compute_cache_key(server);
        if cached.server_name == expected_key {
            Some(cached.clone())
        } else {
            None
        }
    }

    /// Store cached schemas for a server.
    pub async fn put(
        &self,
        server_name: &str,
        server: &McpServer,
        tools: Vec<CachedToolSchema>,
    ) -> anyhow::Result<()> {
        let key = Self::compute_cache_key(server);
        let schemas = CachedServerSchemas {
            server_name: key,
            cached_at: chrono::Utc::now(),
            tools,
        };

        // Update memory
        {
            let mut memory = self.memory.write().await;
            memory.insert(server_name.to_string(), schemas.clone());
        }

        // Persist to disk
        let path = self.cache_dir.join(format!("{}.json", server_name));
        let content = serde_json::to_string_pretty(&schemas)?;
        tokio::fs::write(&path, content).await?;

        Ok(())
    }

    /// Invalidate cache for a specific server.
    pub async fn invalidate(&self, server_name: &str) -> anyhow::Result<()> {
        // Remove from memory
        {
            let mut memory = self.memory.write().await;
            memory.remove(server_name);
        }
        // Remove from disk
        let path = self.cache_dir.join(format!("{}.json", server_name));
        if path.exists() {
            tokio::fs::remove_file(&path).await?;
        }
        Ok(())
    }

    /// Get all cached server names.
    pub async fn cached_servers(&self) -> Vec<String> {
        let memory = self.memory.read().await;
        memory.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_deterministic() {
        let server = McpServer {
            transport: crate::config::McpTransport::Stdio,
            command: vec![
                "npx".to_string(),
                "-y".to_string(),
                "@mcp/server".to_string(),
            ],
            url: None,
            headers: Default::default(),
            scope: crate::config::ConfigScope::Global,
        };

        let key1 = SchemaCache::compute_cache_key(&server);
        let key2 = SchemaCache::compute_cache_key(&server);
        assert_eq!(key1, key2, "cache key should be deterministic");
    }

    #[test]
    fn cache_key_different_for_different_config() {
        let server1 = McpServer {
            transport: crate::config::McpTransport::Stdio,
            command: vec!["npx".to_string()],
            url: None,
            headers: Default::default(),
            scope: crate::config::ConfigScope::Global,
        };

        let server2 = McpServer {
            transport: crate::config::McpTransport::Stdio,
            command: vec!["node".to_string()],
            url: None,
            headers: Default::default(),
            scope: crate::config::ConfigScope::Global,
        };

        let key1 = SchemaCache::compute_cache_key(&server1);
        let key2 = SchemaCache::compute_cache_key(&server2);
        assert_ne!(
            key1, key2,
            "different configs should produce different keys"
        );
    }

    #[test]
    fn cache_key_is_sha256() {
        let server = McpServer::default();
        let key = SchemaCache::compute_cache_key(&server);
        // SHA-256 produces 64 hex characters
        assert_eq!(key.len(), 64);
        assert!(key.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn cached_server_schemas_serialization() {
        let schemas = CachedServerSchemas {
            server_name: "test-key".to_string(),
            cached_at: chrono::Utc::now(),
            tools: vec![CachedToolSchema {
                name: "test_tool".to_string(),
                description: "A test tool".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    }
                }),
            }],
        };

        let json = serde_json::to_string(&schemas).unwrap();
        let parsed: CachedServerSchemas = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.server_name, "test-key");
        assert_eq!(parsed.tools.len(), 1);
        assert_eq!(parsed.tools[0].name, "test_tool");
    }
}
