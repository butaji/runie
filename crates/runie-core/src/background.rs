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
    _owner: Arc<TaskOwner>,
}

impl Default for BackgroundProcessActor {
    fn default() -> Self {
        Self::new()
    }
}

impl BackgroundProcessActor {
    pub fn new() -> Self {
        let (snapshot_tx, snapshot) = watch::channel(Vec::new());
        let (tx, owner) =
            spawn_actor_worker!(
                32,
                move |rx| async move { run_worker(rx, snapshot_tx).await }
            );
        Self {
            tx,
            snapshot,
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
}

async fn run_worker(
    mut rx: mpsc::Receiver<Message>,
    snapshot_tx: watch::Sender<Vec<BackgroundJob>>,
) {
    let mut jobs = BTreeMap::<String, BackgroundJob>::new();
    let mut handles = BTreeMap::new();
    let mut tasks = JoinSet::new();
    let mut next_id = 0_u64;
    loop {
        tokio::select! {
            message = rx.recv() => match message {
                Some(Message::Start { command, reply }) => {
                    let id = next_id.to_string(); next_id += 1;
                    jobs.insert(id.clone(), BackgroundJob { id: id.clone(), command: command.clone(), status: BackgroundStatus::Running, output: String::new() });
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
                        match result { Ok(output) => { job.status = BackgroundStatus::Completed; job.output = output; }, Err(output) => { job.status = BackgroundStatus::Failed; job.output = output; } }
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

fn publish(tx: &watch::Sender<Vec<BackgroundJob>>, jobs: &BTreeMap<String, BackgroundJob>) {
    let _ = tx.send(jobs.values().cloned().collect());
}

async fn run_command(id: String, command: String) -> (String, Result<String, String>) {
    let result = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()
        .await
        .map_err(|error| error.to_string())
        .and_then(|output| {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let text = bounded_output(text);
            if output.status.success() {
                Ok(text)
            } else {
                Err(text)
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

    #[tokio::test]
    async fn failed_jobs_preserve_failure_output_and_status() {
        let actor = BackgroundProcessActor::new();
        let id = actor.start("printf failed >&2; exit 3").await.unwrap();
        loop {
            if let Some(job) = actor.snapshot().into_iter().find(|job| job.id == id) {
                if job.status == BackgroundStatus::Failed {
                    assert_eq!(job.output, "failed");
                    break;
                }
            }
            tokio::task::yield_now().await;
        }
    }
}
