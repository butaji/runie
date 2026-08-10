//! Owned background shell jobs. The actor owns every JoinSet task and snapshot.

use crate::task_owner::{spawn_actor_worker, TaskOwner};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinSet;

pub const BACKGROUND_OUTPUT_MAX_BYTES: usize = 100 * 1024;
const OUTPUT_TRUNCATION_MARKER: &str = "\n[output truncated]";

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
            message = rx.recv() => match message {
                Some(Message::Start { command, reply }) => {
                    let id = next_id.to_string(); next_id += 1;
                    jobs.insert(id.clone(), BackgroundJob { id: id.clone(), command: command.clone(), status: BackgroundStatus::Running, output: String::new(), exit_code: None });
                    handles.insert(id.clone(), tasks.spawn(run_command(id.clone(), command)));
                    publish(&snapshot_tx, &jobs);
                    let _ = reply.send(Ok(id));
                }
                Some(Message::Cancel { id, reply }) => {
                    let cancelled = cancel_job(&id, &mut jobs, &mut handles);
                    if cancelled { publish(&snapshot_tx, &jobs); }
                    let _ = reply.send(cancelled);
                }
                None => break,
            },
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
    use super::*;
    #[tokio::test]
    async fn jobs_are_owned_and_reduce_to_completion() {
        let actor = BackgroundProcessActor::new();
        let id = actor.start("printf done").await.unwrap();
        loop {
            if let Some(job) = actor.snapshot().into_iter().find(|job| job.id == id) {
                if job.status == BackgroundStatus::Completed {
                    assert_eq!(job.output, "done");
                    assert_eq!(actor.shared_snapshot().get()[0].output, "done");
                    assert_eq!(actor.shared_snapshot().strong_count(), 2);
                    break;
                }
            }
            tokio::task::yield_now().await;
        }
    }

    #[test]
    fn output_projection_is_bounded_and_marks_truncation() {
        let output = bounded_output("x".repeat(BACKGROUND_OUTPUT_MAX_BYTES + 1));
        assert!(output.len() <= BACKGROUND_OUTPUT_MAX_BYTES);
        assert!(output.ends_with(OUTPUT_TRUNCATION_MARKER));
    }

    #[test]
    fn terminal_line_preserves_status_exit_and_output_as_data() {
        let job = BackgroundJob {
            id: "3".into(),
            command: "printf done".into(),
            status: BackgroundStatus::Completed,
            output: "done\nnext".into(),
            exit_code: Some(0),
        };
        assert_eq!(
            job.terminal_line(),
            "3 · Completed · printf done exit=0 · done\\nnext"
        );
    }

    #[tokio::test]
    async fn failed_jobs_preserve_failure_output_and_status() {
        let actor = BackgroundProcessActor::new();
        let id = actor.start("printf failed >&2; exit 3").await.unwrap();
        loop {
            if let Some(job) = actor.snapshot().into_iter().find(|job| job.id == id) {
                if job.status == BackgroundStatus::Failed {
                    assert_eq!(job.output, "failed");
                    assert_eq!(job.exit_code, Some(3));
                    break;
                }
            }
            tokio::task::yield_now().await;
        }
    }
}
