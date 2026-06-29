//! Session file header handling.

use crate::session::index::SessionMetadata;
use std::io::{Read, Write};
use std::path::Path;

/// Header stored at the start of each session file.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionHeader {
    pub id: String,
    pub display_name: String,
    pub created_at: f64,
    pub updated_at: f64,
    pub message_count: usize,
    pub summary: Option<String>,
    #[serde(default)]
    pub is_starred: bool,
    #[serde(default)]
    pub is_system: bool,
}

impl From<&SessionMetadata> for SessionHeader {
    fn from(meta: &SessionMetadata) -> Self {
        Self {
            id: meta.id.clone(),
            display_name: meta.display_name.clone(),
            created_at: meta.created_at,
            updated_at: meta.updated_at,
            message_count: meta.message_count,
            summary: meta.summary.clone(),
            is_starred: meta.is_starred,
            is_system: meta.is_system,
        }
    }
}

impl From<&SessionHeader> for SessionMetadata {
    fn from(header: &SessionHeader) -> Self {
        Self {
            id: header.id.clone(),
            display_name: header.display_name.clone(),
            created_at: header.created_at,
            updated_at: header.updated_at,
            message_count: header.message_count,
            summary: header.summary.clone(),
            is_starred: header.is_starred,
            is_system: header.is_system,
        }
    }
}

/// Read the header from a session file.
pub fn read_header(path: &Path) -> anyhow::Result<Option<SessionHeader>> {
    if !path.exists() {
        return Ok(None);
    }
    let mut file = std::fs::File::open(path)?;
    let mut first_line = String::new();
    file.read_to_string(&mut first_line)?;

    let first_line = first_line.trim();
    if first_line.is_empty() {
        return Ok(None);
    }

    match serde_json::from_str::<SessionHeader>(first_line) {
        Ok(header) => Ok(Some(header)),
        Err(_) => Ok(None),
    }
}

/// Write the header to a session file.
pub fn write_header(path: &Path, header: &SessionHeader) -> anyhow::Result<()> {
    let header_line = serde_json::to_string(header)?;
    let content = std::fs::read_to_string(path)?;
    let mut file = std::fs::File::create(path)?;
    writeln!(file, "{}", header_line)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

/// Update the timestamp in a header.
pub fn touch_header(path: &Path) -> anyhow::Result<()> {
    if let Some(mut header) = read_header(path)? {
        header.updated_at = crate::message::now();
        write_header(path, &header)?;
    }
    Ok(())
}
