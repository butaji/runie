//! Session file header handling.
//!
//! The session file header is the first JSON line of each session file.
//! `SessionHeader` is an alias for `SessionMetadata` — the same type is
//! used both in the file header and in the session index.

use std::io::{BufRead, Write};
use std::path::Path;

pub use crate::session::index::SessionMetadata as SessionHeader;

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
