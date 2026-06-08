//! Session persistence via JSONL — one domain event per line.
//!
//! File format:
//!   Line 1:  {"type":"header","version":1,"name":"...","created_at":...,"provider":"...","model":"..."}
//!   Line N:  {"variant":"Submit","content":"..."}
//!
//! Supports streaming read/write for large sessions.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

use crate::event_bus::DomainEvent;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Metadata header
// ---------------------------------------------------------------------------

/// Metadata header stored as the first line of every JSONL session file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    /// Schema version; bump when breaking changes are introduced.
    pub version: u8,
    /// Human-readable session name (used as filename stem).
    pub name: String,
    /// Unix timestamp when the session was created.
    pub created_at: f64,
    /// Unix timestamp of the last recorded event.
    pub updated_at: f64,
    /// LLM provider identifier (e.g. "openai", "mock").
    pub provider: String,
    /// Model name (e.g. "gpt-4o", "echo").
    pub model: String,
}

impl SessionMeta {
    /// Start a new session with the given name and provider/model info.
    pub fn new(name: String, provider: String, model: String) -> Self {
        let now = timestamp();
        Self {
            version: 1,
            name,
            created_at: now,
            updated_at: now,
            provider,
            model,
        }
    }

    /// Bump the updated_at timestamp.
    pub fn touch(&mut self) {
        self.updated_at = timestamp();
    }
}

/// Header discriminant for the first-line JSON object.
const HEADER_TYPE: &str = "header";

fn timestamp() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}
// ---------------------------------------------------------------------------
// JSONL reader
// ---------------------------------------------------------------------------

/// Streaming JSONL reader — lazily parses one line at a time.
pub struct JsonlReader {
    reader: BufReader<File>,
    line_no: usize,
}

impl JsonlReader {
    /// Open a session file for reading.
    pub fn open(path: &PathBuf) -> Result<Self> {
        let file = File::open(path).with_context(|| format!("open session: {:?}", path))?;
        Ok(Self { reader: BufReader::new(file), line_no: 0 })
    }

    /// Read and return the metadata header (first line).
    pub fn read_meta(&mut self) -> Result<SessionMeta> {
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        self.line_no = 1;
        line = line.trim_end().to_string();
        if line.is_empty() {
            anyhow::bail!("empty session file");
        }
        let raw: serde_json::Value =
            serde_json::from_str(&line).with_context(|| "parse header JSON")?;
        let ty = raw.get("type").and_then(|v| v.as_str());
        if ty != Some(HEADER_TYPE) {
            anyhow::bail!("expected header line, got type {:?}", ty);
        }
        serde_json::from_value(raw).with_context(|| "deserialize SessionMeta")
    }

    /// Read the next domain event, or Ok(None) on EOF.
    pub fn read_event(&mut self) -> Result<Option<DomainEvent>> {
        let mut line = String::new();
        match self.reader.read_line(&mut line)? as usize {
            0 => Ok(None),
            _ => {
                self.line_no += 1;
                let line = line.trim_end();
                if line.is_empty() {
                    return self.read_event();
                }
                let raw: serde_json::Value =
                    serde_json::from_str(line).with_context(|| format!("parse JSON at line {}", self.line_no))?;
                let event: DomainEvent =
                    serde_json::from_value(raw).with_context(|| "deserialize DomainEvent")?;
                Ok(Some(event))
            }
        }
    }

    /// Consume the reader and return all events as a vector.
    pub fn read_all_events(&mut self) -> Result<Vec<DomainEvent>> {
        let mut events = Vec::new();
        while let Some(evt) = self.read_event()? {
            events.push(evt);
        }
        Ok(events)
    }

    /// Read header + all events.
    pub fn read_session(&mut self) -> Result<(SessionMeta, Vec<DomainEvent>)> {
        let meta = self.read_meta()?;
        let events = self.read_all_events()?;
        Ok((meta, events))
    }

    /// Number of lines consumed so far (for debugging / error reporting).
    pub fn line_no(&self) -> usize {
        self.line_no
    }
}

// ---------------------------------------------------------------------------
// JSONL writer
// ---------------------------------------------------------------------------

/// Streaming JSONL writer — appends events one line at a time.
pub struct JsonlWriter {
    writer: File,
    path: PathBuf,
}

impl JsonlWriter {
    /// Create (or truncate) a session file and write the metadata header.
    pub fn create(path: &PathBuf, meta: &SessionMeta) -> Result<Self> {
        let file = File::create(path)
            .with_context(|| format!("create session file: {:?}", path))?;
        let mut writer = Self { writer: file, path: path.clone() };
        writer.write_meta(meta)?;
        Ok(writer)
    }

    /// Append to an existing session file (resume).
    pub fn append(path: &PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .with_context(|| format!("open session for append: {:?}", path))?;
        Ok(Self { writer: file, path: path.clone() })
    }

    /// Write the metadata header as the first line.
    fn write_meta(&mut self, meta: &SessionMeta) -> Result<()> {
        let mut value = serde_json::to_value(meta)?;
        if let Some(obj) = value.as_object_mut() {
            obj.insert("type".into(), serde_json::json!(HEADER_TYPE));
        }
        let json = serde_json::to_string(&value)?;
        writeln!(self.writer, "{}", json).context("write session header")?;
        Ok(())
    }


    pub fn write_event(&mut self, event: &DomainEvent) -> Result<()> {
        let json = serde_json::to_string(event)?;
        writeln!(self.writer, "{}", json)
            .context("write domain event")?;
        Ok(())
    }


    pub fn flush(&mut self) -> Result<()> {
        self.writer.flush()?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Convenience helpers
// ---------------------------------------------------------------------------

/// Default sessions directory.
pub fn default_sessions_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("runie").join("sessions"))
}

/// Path for a named session file.
pub fn session_path(name: &str) -> Option<PathBuf> {
    default_sessions_dir().map(|d| d.join(format!("{}.jsonl", name)))
}

/// List all saved session names (stem only, no extension).
pub fn list_session_names() -> Result<Vec<String>> {
    let dir = default_sessions_dir().context("no data directory")?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut names: Vec<String> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.strip_suffix(".jsonl").map(String::from)
        })
        .collect();
    names.sort();
    Ok(names)
}

/// Delete a named session.
pub fn delete_session(name: &str) -> Result<()> {
    let path = session_path(name).context("no sessions directory")?;
    std::fs::remove_file(&path).with_context(|| format!("delete session {:?}", path))?;
    Ok(())
}

/// Load a session by name.
pub fn load_session(name: &str) -> Result<(SessionMeta, Vec<DomainEvent>)> {
    let path = session_path(name).context("no sessions directory")?;
    let mut reader = JsonlReader::open(&path)?;
    reader.read_session().with_context(|| format!("load session {:?}", name))
}

/// Save a session by name with the given events.
pub fn save_session(
    name: &str,
    meta: &SessionMeta,
    events: &[DomainEvent],
) -> Result<()> {
    let dir = default_sessions_dir().context("no sessions directory")?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.jsonl", name));
    let mut writer = JsonlWriter::create(&path, meta)?;
    for event in events {
        writer.write_event(event)?;
    }
    writer.flush()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_bus::DomainEvent;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tmp_path() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("runie_jsonl_test_{}_{}.jsonl", std::process::id(), n))
    }

    fn sample_meta(name: &str) -> SessionMeta {
        SessionMeta {
            version: 1,
            name: name.into(),
            created_at: 1.0,
            updated_at: 1.0,
            provider: "mock".into(),
            model: "echo".into(),
        }
    }

    fn sample_events() -> Vec<DomainEvent> {
        vec![
            DomainEvent::Submit { content: "hello world".into() },
            DomainEvent::SpawnAgent,
            DomainEvent::AgentThinking { id: "t1".into() },
            DomainEvent::AgentResponse { id: "r1".into(), content: "hi!".into() },
            DomainEvent::AgentDone { id: "r1".into() },
        ]
    }

    #[test]
    fn roundtrip_single_session() {
        let path = tmp_path();
        let meta = sample_meta("roundtrip");
        let events = sample_events();

        // Write
        {
            let mut w = JsonlWriter::create(&path, &meta).unwrap();
            for e in &events {
                w.write_event(e).unwrap();
            }
            w.flush().unwrap();
        }

        // Read
        let (got_meta, got_events) = {
            let mut r = JsonlReader::open(&path).unwrap();
            r.read_session().unwrap()
        };

        assert_eq!(got_meta.name, "roundtrip");
        assert_eq!(got_meta.version, 1);
        assert_eq!(got_meta.provider, "mock");
        assert_eq!(got_meta.model, "echo");
        assert_eq!(got_events.len(), events.len());

        // Variant equality
        for (a, b) in events.iter().zip(got_events.iter()) {
            assert_eq!(format!("{:?}", a), format!("{:?}", b));
        }

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn empty_events_list() {
        let path = tmp_path();
        let meta = sample_meta("empty");

        {
            let mut w = JsonlWriter::create(&path, &meta).unwrap();
            w.flush().unwrap();
        }

        let (_meta, events) = {
            let mut r = JsonlReader::open(&path).unwrap();
            r.read_session().unwrap()
        };
        assert!(events.is_empty());
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn domain_event_serde_submit() {
        let evt = DomainEvent::Submit { content: "test".into() };
        let json = serde_json::to_string(&evt).unwrap();
        let decoded: DomainEvent = serde_json::from_str(&json).unwrap();
        match decoded {
            DomainEvent::Submit { content } => assert_eq!(content, "test"),
            _ => panic!("wrong variant"),
        }
    }
    #[test]
    fn domain_event_serde_tool_start() {
        let evt = DomainEvent::AgentToolStart { id: "id1".into(), name: "bash".into() };
        let json = serde_json::to_string(&evt).unwrap();
        let decoded: DomainEvent = serde_json::from_str(&json).unwrap();
        match decoded {
            DomainEvent::AgentToolStart { id, name } => {
                assert_eq!(id, "id1");
                assert_eq!(name, "bash");
            }
            _ => panic!("wrong variant"),
        }
    }
    #[test]
    fn list_session_names_empty() {
        // Uses a temp dir that doesn't exist
        let tmp_dir = std::env::temp_dir().join(format!(
            "runie_list_test_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&tmp_dir);
        // Patch default_sessions_dir via path manipulation isn't easy in tests,
        // so we test the JsonlReader path directly.
        let path = tmp_dir.join("empty.jsonl");
        std::fs::create_dir_all(&tmp_dir).unwrap();
        std::fs::write(&path, r#"{"type":"header","version":1,"name":"empty","created_at":1.0,"updated_at":1.0,"provider":"mock","model":"echo"}"#).unwrap();
        let mut r = JsonlReader::open(&path).unwrap();
        let (meta, events) = r.read_session().unwrap();
        assert_eq!(meta.name, "empty");
        assert!(events.is_empty());
        std::fs::remove_dir_all(&tmp_dir).ok();
    }

    #[test]
    fn read_event_ignores_blank_lines() {
        let path = tmp_path();
        let meta = sample_meta("blank-lines");
        let json_meta = serde_json::to_string(&meta).unwrap();
        let json_evt1 = serde_json::to_string(&DomainEvent::Submit { content: "a".into() }).unwrap();
        let json_evt2 = serde_json::to_string(&DomainEvent::SpawnAgent).unwrap();
        let header = format!(r#"{{"type":"header",{}}}"#, &json_meta[1..json_meta.len()-1]);
        std::fs::write(&path, format!("{}\n\n{}\n  \n{}\n", header, json_evt1, json_evt2)).unwrap();

        let mut r = JsonlReader::open(&path).unwrap();
        let (_meta, events) = r.read_session().unwrap();
        assert_eq!(events.len(), 2);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn append_writes_to_existing_file() {
        let path = tmp_path();
        let meta = sample_meta("append-test");

        // Write header + first event
        {
            let mut w = JsonlWriter::create(&path, &meta).unwrap();
            w.write_event(&DomainEvent::Submit { content: "first".into() }).unwrap();
            w.flush().unwrap();
        }

        // Append second event
        {
            let mut w = JsonlWriter::append(&path).unwrap();
            w.write_event(&DomainEvent::SpawnAgent).unwrap();
            w.flush().unwrap();
        }

        let (_meta, events) = {
            let mut r = JsonlReader::open(&path).unwrap();
            r.read_session().unwrap()
        };
        assert_eq!(events.len(), 2);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn line_numbers_increment() {
        let path = tmp_path();
        let meta = sample_meta("line-nums");
        let json_meta = serde_json::to_string(&meta).unwrap();
        let evt = DomainEvent::Submit { content: "x".into() };
        let json_evt = serde_json::to_string(&evt).unwrap();
        let header = format!(r#"{{"type":"header",{}}}"#, &json_meta[1..json_meta.len()-1]);
        std::fs::write(&path, format!("{}\n{}\n{}\n", header, json_evt, json_evt)).unwrap();

        let mut r = JsonlReader::open(&path).unwrap();
        assert_eq!(r.line_no(), 0);
        r.read_meta().unwrap();
        assert_eq!(r.line_no(), 1);
        r.read_event().unwrap();
        assert_eq!(r.line_no(), 2);
        r.read_event().unwrap();
        assert_eq!(r.line_no(), 3);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn session_meta_serde_roundtrip() {
        let meta = SessionMeta::new("s".into(), "p".into(), "m".into());
        let json = serde_json::to_string(&meta).unwrap();
        let dec: SessionMeta = serde_json::from_str(&json).unwrap();
        assert_eq!(dec.name, "s");
    }

    #[test]
    fn domain_event_all_variants() {
        use DomainEvent::*;
        let variants: Vec<DomainEvent> = vec![
            Submit { content: "c".into() },
            SpawnAgent,
            AgentThinking { id: "id".into() },
            AgentThoughtDone { id: "id".into() },
            AgentResponse { id: "id".into(), content: "c".into() },
            AgentTurnComplete { id: "id".into(), duration_secs: 1.5 },
            AgentToolStart { id: "id".into(), name: "bash".into() },
            AgentToolEnd { id: "id".into(), name: "bash".into(), duration_secs: 0.5, output: "out".into() },
            AgentDone { id: "id".into() },
            AgentError { id: "id".into(), message: "err".into() },
            SwitchModel { provider: "openai".into(), model: "gpt-4o".into() },
            FollowUp { content: "c".into() },
            ToolRegistered { name: "bash".into() },
        ];
        for evt in variants {
            let json = serde_json::to_string(&evt).unwrap();
            let decoded: DomainEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(
                format!("{:?}", evt),
                format!("{:?}", decoded),
                "roundtrip failed for {:?}",
                evt
            );
        }
    }
}
