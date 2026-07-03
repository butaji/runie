//! Session file header handling.
//!
//! The session file header is the first JSON line of each session file.
//! `SessionHeader` is an alias for `SessionMetadata` (defined in `crate::session::mod.rs`).

use std::io::BufRead;
use std::path::Path;

pub use crate::session::SessionMetadata as SessionHeader;

use crate::io::atomic_write::atomic_write;

/// Read the header from a session file.
pub fn read_header(path: &Path) -> anyhow::Result<Option<SessionHeader>> {
    if !path.exists() {
        return Ok(None);
    }
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);

    let mut first_line = String::new();
    match reader.read_line(&mut first_line) {
        Ok(0) => return Ok(None), // Empty file
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e.into()),
        Ok(_) => {}
    }

    let first_line = first_line.trim();
    if first_line.is_empty() {
        return Ok(None);
    }

    match serde_json::from_str::<SessionHeader>(first_line) {
        Ok(header) => Ok(Some(header)),
        Err(_) => Ok(None),
    }
}

/// Write the header to a session file atomically.
///
/// Reads the existing content, prepends the new header as the first JSON line,
/// and atomically replaces the file using `atomic_write`.
pub fn write_header(path: &Path, header: &SessionHeader) -> anyhow::Result<()> {
    let header_line = serde_json::to_string(header)?;
    let content = std::fs::read_to_string(path)?;
    // Build the new file content: header as first line, then the rest
    let new_content = format!("{}\n{}", header_line, content);
    atomic_write(path, &new_content)?;
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
