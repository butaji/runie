//! Owned background shell jobs. The actor owns every JoinSet task and snapshot.

use crate::output::{bounded_preview, output_facts};
use crate::task_owner::{spawn_actor_worker, TaskOwner};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinSet;

#[path = "background_controls.rs"]
mod controls;

pub const BACKGROUND_OUTPUT_MAX_BYTES: usize = 100 * 1024;
const OUTPUT_TRUNCATION_MARKER: &str = "\n[output truncated]";
const BACKGROUND_PREVIEW_MAX_CHARS: usize = 256;
type BackgroundTask = (String, Result<(String, Option<i32>), (String, Option<i32>)>);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackgroundStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundJob {
    pub id: String,
    pub command: String,
    pub status: BackgroundStatus,
    pub output: String,
    #[serde(default)]
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundJobSummary {
    pub id: String,
    pub command: String,
    pub status: BackgroundStatus,
    pub exit_code: Option<i32>,
    pub output_lines: usize,
    pub output_bytes: usize,
    pub output_preview: Option<String>,
    pub truncated: bool,
}

impl BackgroundJobSummary {
    pub fn terminal_line(&self) -> String {
        format!(
            "{} · {:?} · {} exit={:?} · output={} lines/{} bytes{}{}",
            self.id,
            self.status,
            self.command,
            self.exit_code,
            self.output_lines,
            self.output_bytes,
            if self.truncated { " truncated" } else { "" },
            self.output_preview
                .as_deref()
                .map(|preview| format!(" · preview={preview:?}"))
                .unwrap_or_default(),
        )
    }
}

pub fn background_job_summaries(jobs: &[BackgroundJob]) -> Vec<BackgroundJobSummary> {
    jobs.iter()
        .map(|job| {
            let facts = output_facts(&job.output, job.output.contains(OUTPUT_TRUNCATION_MARKER));
            BackgroundJobSummary {
                id: job.id.clone(),
                command: job.command.clone(),
                status: job.status.clone(),
                exit_code: job.exit_code,
                output_lines: facts.lines,
                output_bytes: facts.bytes,
                output_preview: bounded_preview(&job.output, BACKGROUND_PREVIEW_MAX_CHARS),
                truncated: facts.truncated,
            }
        })
        .collect()
}

impl BackgroundJob {
    pub fn terminal_line(&self) -> String {
        let exit = self
            .exit_code
            .map(|code| format!(" exit={code}"))
            .unwrap_or_default();
        let output = if self.output.is_empty() {
            String::new()
        } else {
            format!(" · {}", self.output.replace('\n', "\\n"))
        };
        format!(
            "{} · {:?} · {}{}{}",
            self.id, self.status, self.command, exit, output
        )
    }
}

enum Message {
    Start {
        command: String,
        reply: oneshot::Sender<Result<String, String>>,
    },
    Cancel {
        id: String,
        reply: oneshot::Sender<bool>,
    },
    CancelAll {
        reply: oneshot::Sender<usize>,
    },
    ClearFinished {
        reply: oneshot::Sender<usize>,
    },
    ReadOutput {
        id: String,
        reply: oneshot::Sender<Option<String>>,
    },
}

#[derive(Clone)]
pub struct BackgroundProcessActor {
    tx: mpsc::Sender<Message>,
    snapshot: watch::Receiver<Vec<BackgroundJob>>,
    shared_snapshot: watch::Receiver<crate::SharedSnapshot<Vec<BackgroundJob>>>,
    _owner: Arc<TaskOwner>,
}

impl Default for BackgroundProcessActor {
    fn default() -> Self {
        Self::new()
    }
}

impl BackgroundProcessActor {
    pub fn new() -> Self {
        let initial = Vec::new();
        let (snapshot_tx, snapshot) = watch::channel(initial.clone());
        let (shared_tx, shared_snapshot) = watch::channel(crate::SharedSnapshot::new(initial));
        let (tx, owner) = spawn_actor_worker!(32, move |rx| async move {
            run_worker(
                rx,
                BackgroundSnapshotPublisher {
                    snapshot_tx,
                    shared_tx,
                },
            )
            .await
        });
        Self {
            tx,
            snapshot,
            shared_snapshot,
            _owner: owner,
        }
    }
    pub async fn start(&self, command: impl Into<String>) -> Result<String, String> {
        let (reply, result) = oneshot::channel();
        self.tx
            .send(Message::Start {
                command: command.into(),
                reply,
            })
            .await
            .map_err(|_| "background actor is closed".to_owned())?;
        result
            .await
            .map_err(|_| "background actor dropped the start result".to_owned())?
    }
    pub async fn cancel(&self, id: impl Into<String>) -> bool {
        let (reply, result) = oneshot::channel();
        if self
            .tx
            .send(Message::Cancel {
                id: id.into(),
                reply,
            })
            .await
            .is_err()
        {
            return false;
        }
        result.await.unwrap_or(false)
    }

    pub async fn cancel_all(&self) -> usize {
        let (reply, result) = oneshot::channel();
        if self.tx.send(Message::CancelAll { reply }).await.is_err() {
            return 0;
        }
        result.await.unwrap_or_default()
    }

    pub async fn clear_finished(&self) -> usize {
        let (reply, result) = oneshot::channel();
        if self
            .tx
            .send(Message::ClearFinished { reply })
            .await
            .is_err()
        {
            return 0;
        }
        result.await.unwrap_or_default()
    }
    /// Read one job's already-bounded captured output through the owner.
    pub async fn read_output(&self, id: impl Into<String>) -> Option<String> {
        let (reply, result) = oneshot::channel();
        if self
            .tx
            .send(Message::ReadOutput {
                id: id.into(),
                reply,
            })
            .await
            .is_err()
        {
            return None;
        }
        result.await.unwrap_or_default()
    }
    pub fn snapshot(&self) -> Vec<BackgroundJob> {
        self.snapshot.borrow().clone()
    }
    pub fn subscribe(&self) -> watch::Receiver<Vec<BackgroundJob>> {
        self.snapshot.clone()
    }

    pub fn shared_snapshot(&self) -> crate::SharedSnapshot<Vec<BackgroundJob>> {
        self.shared_snapshot.borrow().clone()
    }

    pub fn shared_subscribe(&self) -> watch::Receiver<crate::SharedSnapshot<Vec<BackgroundJob>>> {
        self.shared_snapshot.clone()
    }
}

#[derive(Clone)]
struct BackgroundSnapshotPublisher {
    snapshot_tx: watch::Sender<Vec<BackgroundJob>>,
    shared_tx: watch::Sender<crate::SharedSnapshot<Vec<BackgroundJob>>>,
}

impl BackgroundSnapshotPublisher {
    fn send(&self, jobs: Vec<BackgroundJob>) {
        crate::publish_shared_snapshot(&self.snapshot_tx, &self.shared_tx, jobs);
    }
}

async fn run_worker(mut rx: mpsc::Receiver<Message>, snapshot_tx: BackgroundSnapshotPublisher) {
    let mut jobs = BTreeMap::<String, BackgroundJob>::new();
    let mut handles = BTreeMap::new();
    let mut tasks = JoinSet::new();
    let mut next_id = 0_u64;
    loop {
        tokio::select! {
            message = rx.recv() => if !handle_message(message, &mut jobs, &mut handles, &mut tasks, &snapshot_tx, &mut next_id) { break },
            result = tasks.join_next(), if !tasks.is_empty() => {
                if let Some(Ok((id, result))) = result {
                    handles.remove(&id);
                    if let Some(job) = jobs.get_mut(&id) {
                        match result { Ok((output, code)) => { job.status = BackgroundStatus::Completed; job.output = output; job.exit_code = code; }, Err((output, code)) => { job.status = BackgroundStatus::Failed; job.output = output; job.exit_code = code; } }
                    }
                    publish(&snapshot_tx, &jobs);
                }
            }
        }
    }
}

fn handle_message(
    message: Option<Message>,
    jobs: &mut BTreeMap<String, BackgroundJob>,
    handles: &mut BTreeMap<String, tokio::task::AbortHandle>,
    tasks: &mut JoinSet<BackgroundTask>,
    publisher: &BackgroundSnapshotPublisher,
    next_id: &mut u64,
) -> bool {
    match message {
        Some(Message::Start { command, reply }) => {
            handle_start(command, reply, jobs, handles, tasks, publisher, next_id)
        }
        Some(Message::Cancel { id, reply }) => handle_cancel(id, reply, jobs, handles, publisher),
        Some(Message::CancelAll { reply }) => handle_cancel_all(reply, jobs, handles, publisher),
        Some(Message::ClearFinished { reply }) => controls::clear_finished(reply, jobs, publisher),
        Some(Message::ReadOutput { id, reply }) => {
            let _ = reply.send(jobs.get(&id).map(|job| job.output.clone()));
        }
        None => return false,
    }
    true
}

fn handle_start(
    command: String,
    reply: oneshot::Sender<Result<String, String>>,
    jobs: &mut BTreeMap<String, BackgroundJob>,
    handles: &mut BTreeMap<String, tokio::task::AbortHandle>,
    tasks: &mut JoinSet<BackgroundTask>,
    publisher: &BackgroundSnapshotPublisher,
    next_id: &mut u64,
) {
    let id = next_id.to_string();
    *next_id += 1;
    jobs.insert(
        id.clone(),
        BackgroundJob {
            id: id.clone(),
            command: command.clone(),
            status: BackgroundStatus::Running,
            output: String::new(),
            exit_code: None,
        },
    );
    handles.insert(id.clone(), tasks.spawn(run_command(id.clone(), command)));
    publish(publisher, jobs);
    let _ = reply.send(Ok(id));
}

fn handle_cancel(
    id: String,
    reply: oneshot::Sender<bool>,
    jobs: &mut BTreeMap<String, BackgroundJob>,
    handles: &mut BTreeMap<String, tokio::task::AbortHandle>,
    publisher: &BackgroundSnapshotPublisher,
) {
    let cancelled = cancel_job(&id, jobs, handles);
    if cancelled {
        publish(publisher, jobs);
    }
    let _ = reply.send(cancelled);
}

fn handle_cancel_all(
    reply: oneshot::Sender<usize>,
    jobs: &mut BTreeMap<String, BackgroundJob>,
    handles: &mut BTreeMap<String, tokio::task::AbortHandle>,
    publisher: &BackgroundSnapshotPublisher,
) {
    let ids = jobs
        .values()
        .filter(|job| job.status == BackgroundStatus::Running)
        .map(|job| job.id.clone())
        .collect::<Vec<_>>();
    let cancelled = ids
        .iter()
        .filter(|id| cancel_job(id, jobs, handles))
        .count();
    if cancelled > 0 {
        publish(publisher, jobs);
    }
    let _ = reply.send(cancelled);
}

fn cancel_job(
    id: &str,
    jobs: &mut BTreeMap<String, BackgroundJob>,
    handles: &mut BTreeMap<String, tokio::task::AbortHandle>,
) -> bool {
    if !jobs
        .get(id)
        .is_some_and(|job| job.status == BackgroundStatus::Running)
    {
        return false;
    }
    if let Some(handle) = handles.remove(id) {
        handle.abort();
    }
    if let Some(job) = jobs.get_mut(id) {
        job.status = BackgroundStatus::Cancelled;
    }
    true
}

fn publish(tx: &BackgroundSnapshotPublisher, jobs: &BTreeMap<String, BackgroundJob>) {
    tx.send(jobs.values().cloned().collect());
}

async fn run_command(
    id: String,
    command: String,
) -> (String, Result<(String, Option<i32>), (String, Option<i32>)>) {
    let result = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()
        .await
        .map_err(|error| (error.to_string(), None))
        .and_then(|output| {
            let code = output.status.code();
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let text = bounded_output(text);
            if output.status.success() {
                Ok((text, code))
            } else {
                Err((text, code))
            }
        });
    (id, result)
}

fn bounded_output(mut output: String) -> String {
    if output.len() <= BACKGROUND_OUTPUT_MAX_BYTES {
        return output;
    }
    let keep = BACKGROUND_OUTPUT_MAX_BYTES.saturating_sub(OUTPUT_TRUNCATION_MARKER.len());
    output.truncate(keep);
    output.push_str(OUTPUT_TRUNCATION_MARKER);
    output
}

#[cfg(test)]
mod tests {
    include!("background_tests.inc");
}
