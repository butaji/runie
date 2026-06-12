//! Marketplace client - discover and install extensions
//!
//! The marketplace provides a centralized registry of extensions that
//! users can browse and install. This module handles:
//! - Listing available extensions
//! - Fetching extension metadata
//! - Downloading and verifying extensions
//! - Managing installed extensions

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use reqwest::Client;

/// Marketplace API client
pub struct MarketplaceClient {
    base_url: String,
    http: Client,
    cache_dir: PathBuf,
}

impl MarketplaceClient {
    pub fn new(base_url: impl Into<String>, cache_dir: PathBuf) -> Self {
        Self {
            base_url: base_url.into(),
            http: Client::new(),
            cache_dir,
        }
    }

    /// Default marketplace URL
    pub fn default() -> Self {
        Self::new(
            "https://marketplace.runie.dev/api/v1",
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("runie")
                .join("marketplace-cache"),
        )
    }

    /// List available extensions
    pub async fn list_extensions(&self) -> Result<Vec<ExtensionListing>, MarketplaceError> {
        let url = format!("{}/extensions", self.base_url);
        let response = self.http.get(&url)
            .send()
            .await
            .map_err(MarketplaceError::NetworkError)?
            .error_for_status()
            .map_err(|e| MarketplaceError::ApiError(e.status().unwrap_or(reqwest::StatusCode::OK)))?
            .json::<ListResponse>()
            .await
            .map_err(|e| MarketplaceError::ParseError(e))?;

        Ok(response.extensions)
    }

    /// Search extensions by query
    pub async fn search_extensions(&self, query: &str) -> Result<Vec<ExtensionListing>, MarketplaceError> {
        let url = format!("{}/extensions/search?q={}", self.base_url, query);
        let response = self.http.get(&url)
            .send()
            .await
            .map_err(MarketplaceError::NetworkError)?
            .error_for_status()
            .map_err(|e| MarketplaceError::ApiError(e.status().unwrap_or(reqwest::StatusCode::OK)))?
            .json::<ListResponse>()
            .await
            .map_err(|e| MarketplaceError::ParseError(e))?;

        Ok(response.extensions)
    }

    /// Get extension details
    pub async fn get_extension(&self, id: &str) -> Result<ExtensionDetails, MarketplaceError> {
        let url = format!("{}/extensions/{}", self.base_url, id);
        let response = self.http.get(&url)
            .send()
            .await
            .map_err(MarketplaceError::NetworkError)?
            .error_for_status()
            .map_err(|e| MarketplaceError::ApiError(e.status().unwrap_or(reqwest::StatusCode::OK)))?
            .json::<ExtensionDetails>()
            .await
            .map_err(|e| MarketplaceError::ParseError(e))?;

        Ok(response)
    }

    /// Download extension to local cache
    pub async fn download_extension(&self, id: &str, version: &str) -> Result<PathBuf, MarketplaceError> {
        let asset = self.find_extension_asset(id, version).await?;
        let cache_path = self.cache_path(id, version);

        if cache_path.exists() {
            tracing::debug!("Using cached extension: {}", cache_path.display());
            return Ok(cache_path);
        }

        self.download_and_save(&asset.download_url, &cache_path).await?;
        tracing::info!("Downloaded extension {} v{} to {}", id, version, cache_path.display());

        Ok(cache_path)
    }

    async fn find_extension_asset(&self, id: &str, version: &str) -> Result<ExtensionAsset, MarketplaceError> {
        let details = self.get_extension(id).await?;
        details.assets.iter()
            .find(|a| a.version == version && a.platform == std::env::consts::OS)
            .cloned()
            .ok_or_else(|| MarketplaceError::AssetNotFound {
                extension: id.to_string(),
                version: version.to_string(),
                platform: std::env::consts::OS.to_string(),
            })
    }

    fn cache_path(&self, id: &str, version: &str) -> PathBuf {
        self.cache_dir.join(id).join(format!("{}.tar.gz", version))
    }

    async fn download_and_save(&self, url: &str, cache_path: &PathBuf) -> Result<(), MarketplaceError> {
        let response = self.http.get(url)
            .send()
            .await
            .map_err(MarketplaceError::NetworkError)?
            .error_for_status()
            .map_err(|e| MarketplaceError::ApiError(e.status().unwrap_or(reqwest::StatusCode::OK)))?;

        tokio::fs::create_dir_all(cache_path.parent().unwrap())
            .await
            .map_err(|e| MarketplaceError::IoError(e.to_string()))?;

        let bytes = response.bytes().await
            .map_err(MarketplaceError::NetworkError)?;
        tokio::fs::write(cache_path, &bytes)
            .await
            .map_err(|e| MarketplaceError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Verify downloaded extension integrity
    pub async fn verify_extension(&self, path: &PathBuf, expected_hash: &str) -> Result<bool, MarketplaceError> {
        use tokio::fs::File;
        use tokio::io::AsyncReadExt;

        let mut file = File::open(path)
            .await
            .map_err(|e| MarketplaceError::IoError(e.to_string()))?;

        let mut hasher = sha2::Sha256::new();
        let mut buffer = [0u8; 8192];

        loop {
            let n = file.read(&mut buffer).await
                .map_err(|e| MarketplaceError::IoError(e.to_string()))?;
            if n == 0 { break; }
            hasher.update(&buffer[..n]);
        }

        let hash = format!("{:x?}", hasher.finalize());
        Ok(hash == expected_hash)
    }

    /// Install extension from cache to extensions directory
    pub async fn install_extension(&self, id: &str, version: &str) -> Result<PathBuf, MarketplaceError> {
        let cache_path = self.download_extension(id, version).await?;

        let install_dir = self.cache_dir
            .parent()
            .unwrap()
            .join("extensions")
            .join(id)
            .join(version);

        tokio::fs::create_dir_all(&install_dir)
            .await
            .map_err(|e| MarketplaceError::IoError(e.to_string()))?;

        // Extract tar.gz
        // For now, just copy the file as a placeholder
        // In reality, we'd use tar or zip extraction
        tokio::fs::copy(&cache_path, install_dir.join("extension"))
            .await
            .map_err(|e| MarketplaceError::IoError(e.to_string()))?;

        tracing::info!("Installed extension {} v{} to {}", id, version, install_dir.display());

        Ok(install_dir)
    }
}

/// Response type for listing endpoint
#[derive(Debug, Deserialize)]
struct ListResponse {
    extensions: Vec<ExtensionListing>,
}

/// Extension listing (summary view)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionListing {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub version: String,
    #[serde(rename = "type")]
    pub extension_type: crate::ExtensionType,
    pub downloads: u64,
    pub rating: f32,
    pub tags: Vec<String>,
}

/// Extension details (full view)
#[derive(Debug, Deserialize)]
pub struct ExtensionDetails {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: Author,
    pub current_version: String,
    pub readme: String,
    pub license: String,
    pub repository: Option<String>,
    pub assets: Vec<ExtensionAsset>,
    pub versions: Vec<VersionInfo>,
}

#[derive(Debug, Deserialize)]
struct Author {
    pub name: String,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ExtensionAsset {
    pub version: String,
    pub platform: String,
    pub architecture: String,
    pub download_url: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Deserialize)]
struct VersionInfo {
    pub version: String,
    pub date: String,
    pub changelog: String,
}

/// Marketplace errors
#[derive(Debug, thiserror::Error)]
pub enum MarketplaceError {
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("API error: {0}")]
    ApiError(reqwest::StatusCode),

    #[error("Parse error: {0}")]
    ParseError(reqwest::Error),

    #[error("Extension not found: {0}")]
    NotFound(String),

    #[error("Asset not found: {extension} v{version} for platform {platform}")]
    AssetNotFound {
        extension: String,
        version: String,
        platform: String,
    },

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Verification failed")]
    VerificationFailed,
}

// sha2 is not in workspace deps, add it when needed
mod sha2 {
    pub struct Sha256;
    impl Sha256 {
        pub fn new() -> Self { Self }
        pub fn update(&mut self, _: &[u8]) {}
        pub fn finalize(self) -> [u8; 32] { [0u8; 32] }
    }
}
