use super::*;
enum StorageCommand {
    Discover {
        root: String,
        limit: usize,
        reply: oneshot::Sender<Result<Vec<SessionStorageEntry>, String>>,
    },
    Publish {
        path: String,
        contents: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Load {
        path: String,
        reply: oneshot::Sender<Result<(String, String, SessionSnapshot), String>>,
    },
    Fork {
        path: String,
        snapshot: Box<SessionSnapshot>,
        lane: Option<String>,
        target_id: String,
        session_id: String,
        created_at: i64,
        cwd: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionStorageEntry {
    pub path: String,
    pub session_id: String,
    pub created_at: i64,
    pub cwd: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionStorageRow {
    pub session_id: String,
    pub label: String,
    pub cwd: String,
    pub created_at: i64,
}

impl SessionStorageEntry {
    pub fn row(&self) -> SessionStorageRow {
        SessionStorageRow {
            session_id: self.session_id.clone(),
            label: self
                .cwd
                .rsplit(std::path::MAIN_SEPARATOR)
                .find(|part| !part.is_empty())
                .unwrap_or(&self.session_id)
                .to_owned(),
            cwd: self.cwd.clone(),
            created_at: self.created_at,
        }
    }
}

/// Actor-owned atomic JSONL publication. Serialization stays outside this
/// actor; no caller can observe a partially written destination file.
pub struct SessionStorageActor {
    tx: mpsc::Sender<StorageCommand>,
    _owner: Arc<TaskOwner>,
}

async fn run_storage_worker(mut rx: mpsc::Receiver<StorageCommand>) {
    while let Some(command) = rx.recv().await {
        dispatch_storage_command(command).await;
    }
}

async fn dispatch_storage_command(command: StorageCommand) {
    match command {
        StorageCommand::Discover { root, limit, reply } => {
            let _ = reply.send(discover_storage_files(&root, limit).await);
        }
        StorageCommand::Publish {
            path,
            contents,
            reply,
        } => {
            let _ = reply.send(publish_storage_file(&path, contents).await);
        }
        StorageCommand::Load { path, reply } => {
            let _ = reply.send(load_storage_file(&path).await);
        }
        StorageCommand::Fork {
            path,
            snapshot,
            lane,
            target_id,
            session_id,
            created_at,
            cwd,
            reply,
        } => {
            let _ = reply.send(
                fork_storage_file(
                    &path,
                    *snapshot,
                    lane,
                    &target_id,
                    &session_id,
                    created_at,
                    &cwd,
                )
                .await,
            );
        }
    }
}

async fn discover_storage_files(
    root: &str,
    limit: usize,
) -> Result<Vec<SessionStorageEntry>, String> {
    let mut entries = tokio::fs::read_dir(root)
        .await
        .map_err(|error| format!("discover session directory: {error}"))?;
    let mut found = Vec::new();
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|error| format!("read session directory entry: {error}"))?
    {
        if !entry
            .file_type()
            .await
            .map(|t| t.is_file())
            .unwrap_or(false)
        {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        if let Some(metadata) = read_storage_header(&path).await? {
            found.push(metadata);
        }
    }
    found.sort_by(|left, right| left.path.cmp(&right.path));
    found.truncate(limit);
    Ok(found)
}

async fn read_storage_header(
    path: &std::path::Path,
) -> Result<Option<SessionStorageEntry>, String> {
    let contents = tokio::fs::read_to_string(path)
        .await
        .map_err(|error| format!("read session header: {error}"))?;
    let Some(line) = contents.lines().next() else {
        return Ok(None);
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return Ok(None);
    };
    if value.get("kind").and_then(|v| v.as_str()) != Some("header") {
        return Ok(None);
    }
    let Some(session_id) = value.get("id").and_then(|v| v.as_str()) else {
        return Ok(None);
    };
    Ok(Some(SessionStorageEntry {
        path: path.to_string_lossy().into_owned(),
        session_id: session_id.to_owned(),
        created_at: value
            .get("createdAt")
            .and_then(|v| v.as_i64())
            .unwrap_or_default(),
        cwd: value
            .get("cwd")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned(),
    }))
}

async fn publish_storage_file(path: &str, contents: String) -> Result<(), String> {
    let temporary = format!("{path}.tmp");
    tokio::fs::write(&temporary, contents)
        .await
        .map_err(|error| format!("stage session JSONL: {error}"))?;
    rename_storage_file(&temporary, path, "publish session JSONL").await
}

async fn load_storage_file(path: &str) -> Result<(String, String, SessionSnapshot), String> {
    let contents = tokio::fs::read_to_string(path)
        .await
        .map_err(|error| format!("read session JSONL: {error}"))?;
    let repaired = SessionSnapshot::repair_jsonl_torn_tail(&contents)?;
    SessionSnapshot::from_jsonl(&repaired)
}

async fn fork_storage_file(
    path: &str,
    snapshot: SessionSnapshot,
    lane: Option<String>,
    target_id: &str,
    session_id: &str,
    created_at: i64,
    cwd: &str,
) -> Result<(), String> {
    let fork = match lane.as_deref() {
        Some(lane) => snapshot.fork_at_lane_message(lane, target_id)?,
        None => snapshot.fork_at_message(target_id)?,
    };
    let temporary = format!("{path}.tmp");
    tokio::fs::write(&temporary, fork.to_jsonl(session_id, created_at, cwd))
        .await
        .map_err(|error| format!("stage forked session JSONL: {error}"))?;
    rename_storage_file(&temporary, path, "publish forked session JSONL").await
}

async fn rename_storage_file(temporary: &str, path: &str, operation: &str) -> Result<(), String> {
    if let Err(error) = tokio::fs::rename(temporary, path).await {
        let _ = tokio::fs::remove_file(temporary).await;
        return Err(format!("{operation}: {error}"));
    }
    Ok(())
}

impl SessionStorageActor {
    #[allow(
        clippy::too_many_lines,
        reason = "storage mailbox keeps publication and recovery commands explicit"
    )]
    pub fn new() -> Self {
        let (tx, owner) = spawn_actor_worker!(8, |rx: mpsc::Receiver<StorageCommand>| async move {
            run_storage_worker(rx).await;
        });
        Self { tx, _owner: owner }
    }

    pub async fn publish_snapshot(
        &self,
        path: impl Into<String>,
        snapshot: &SessionSnapshot,
        session_id: &str,
        created_at: i64,
        cwd: &str,
    ) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(StorageCommand::Publish {
                path: path.into(),
                contents: snapshot.to_jsonl(session_id, created_at, cwd),
                reply: reply_tx,
            })
            .await
            .map_err(|_| "session storage actor is closed".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session storage response was dropped".to_owned())?
    }

    /// Discover bounded, valid session headers through the storage owner.
    /// Filesystem traversal and parsing never enter the TUI actor.
    pub async fn discover(
        &self,
        root: impl Into<String>,
        limit: usize,
    ) -> Result<Vec<SessionStorageEntry>, String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(StorageCommand::Discover {
                root: root.into(),
                limit,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "session storage actor is closed".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session storage response was dropped".to_owned())?
    }

    /// Discover and query renderer-neutral picker rows through the storage
    /// owner. Filesystem access and row filtering remain outside UI state.
    pub async fn discover_rows(
        &self,
        root: impl Into<String>,
        limit: usize,
        query: &str,
    ) -> Result<Vec<SessionStorageRow>, String> {
        let entries = self.discover(root, limit).await?;
        let rows = entries
            .into_iter()
            .map(|entry| entry.row())
            .collect::<Vec<_>>();
        Ok(crate::filter_storage_rows(&rows, query))
    }

    pub async fn load_snapshot(
        &self,
        path: impl Into<String>,
    ) -> Result<(String, String, SessionSnapshot), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(StorageCommand::Load {
                path: path.into(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| "session storage actor is closed".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session storage response was dropped".to_owned())?
    }

    pub async fn fork_snapshot(
        &self,
        path: impl Into<String>,
        snapshot: &SessionSnapshot,
        target_id: &str,
        session_id: &str,
        created_at: i64,
        cwd: &str,
    ) -> Result<(), String> {
        self.fork_snapshot_in_lane(path, snapshot, None, target_id, session_id, created_at, cwd)
            .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the storage fork boundary keeps destination, source, lane, and Pi session metadata explicit"
    )]
    pub async fn fork_snapshot_in_lane(
        &self,
        path: impl Into<String>,
        snapshot: &SessionSnapshot,
        lane: Option<&str>,
        target_id: &str,
        session_id: &str,
        created_at: i64,
        cwd: &str,
    ) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(StorageCommand::Fork {
                path: path.into(),
                snapshot: Box::new(snapshot.clone()),
                lane: lane.map(str::to_owned),
                target_id: target_id.to_owned(),
                session_id: session_id.to_owned(),
                created_at,
                cwd: cwd.to_owned(),
                reply: reply_tx,
            })
            .await
            .map_err(|_| "session storage actor is closed".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session storage response was dropped".to_owned())?
    }
}

impl Default for SessionStorageActor {
    fn default() -> Self {
        Self::new()
    }
}
