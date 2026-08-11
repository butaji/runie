use super::{classify_conflicts, plan_conflict_recovery, GitConflictRecoveryState, GitStatusTool};
use crate::types::{AgentTool, ToolResultContent};
use tokio::sync::{mpsc, oneshot, watch};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GitConflictSnapshot {
    pub state: GitConflictRecoveryState,
}

impl GitConflictSnapshot {
    pub fn terminal_lines(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "Git conflicts: {} recoverable={}",
            self.state.plan.paths.len(),
            !self.state.plan.actions.is_empty()
        )];
        lines.extend(self.state.terminal_lines());
        lines
    }
}

enum GitConflictCommand {
    Refresh {
        reply: oneshot::Sender<Result<(), String>>,
    },
    SelectPath {
        path: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
}

#[derive(Clone)]
pub struct GitConflictActor {
    tx: mpsc::Sender<GitConflictCommand>,
    snapshot: watch::Receiver<GitConflictSnapshot>,
    _owner: std::sync::Arc<crate::task_owner::TaskOwner>,
}

impl Default for GitConflictActor {
    fn default() -> Self {
        Self::new()
    }
}

impl GitConflictActor {
    pub fn new() -> Self {
        let initial = GitConflictSnapshot {
            state: super::begin_conflict_recovery(plan_conflict_recovery(&classify_conflicts(""))),
        };
        let (snapshot_tx, snapshot) = watch::channel(initial);
        let (tx, owner) = crate::spawn_actor_worker!(32, move |rx: mpsc::Receiver<
            GitConflictCommand,
        >| async move {
            run_worker(rx, snapshot_tx).await;
        });
        Self {
            tx,
            snapshot,
            _owner: owner,
        }
    }

    pub fn snapshot(&self) -> GitConflictSnapshot {
        self.snapshot.borrow().clone()
    }

    pub async fn refresh(&self) -> Result<(), String> {
        self.request(|reply| GitConflictCommand::Refresh { reply })
            .await
    }

    pub async fn select_path(&self, path: impl Into<String>) -> Result<(), String> {
        self.request(|reply| GitConflictCommand::SelectPath {
            path: path.into(),
            reply,
        })
        .await
    }

    async fn request<F>(&self, command: F) -> Result<(), String>
    where
        F: FnOnce(oneshot::Sender<Result<(), String>>) -> GitConflictCommand,
    {
        let (reply, response) = oneshot::channel();
        self.tx
            .send(command(reply))
            .await
            .map_err(|_| "Git conflict actor is closed".to_owned())?;
        response
            .await
            .map_err(|_| "Git conflict actor response was dropped".to_owned())?
    }
}

async fn run_worker(
    mut rx: mpsc::Receiver<GitConflictCommand>,
    snapshot_tx: watch::Sender<GitConflictSnapshot>,
) {
    let mut state = snapshot_tx.borrow().clone();
    while let Some(command) = rx.recv().await {
        if apply_command(&mut state, command).await {
            let _ = snapshot_tx.send(state.clone());
        }
    }
}

async fn apply_command(state: &mut GitConflictSnapshot, command: GitConflictCommand) -> bool {
    match command {
        GitConflictCommand::Refresh { reply } => {
            let result = refresh_state(state).await;
            let changed = result.is_ok();
            let _ = reply.send(result);
            changed
        }
        GitConflictCommand::SelectPath { path, reply } => {
            let result = super::reduce_conflict_recovery(
                state.state.clone(),
                super::GitConflictRecoveryEvent::PathSelected { path },
            );
            let changed = result.is_ok();
            let _ = reply.send(result.map(|next| {
                state.state = next;
            }));
            changed
        }
    }
}

async fn refresh_state(snapshot: &mut GitConflictSnapshot) -> Result<(), String> {
    let result = GitStatusTool
        .execute("git-conflicts", serde_json::json!({}), None, None)
        .await?;
    let text = result
        .content
        .into_iter()
        .find_map(|content| match content {
            ToolResultContent::Text { text } => Some(text),
            ToolResultContent::Image { .. } => None,
        })
        .unwrap_or_default();
    snapshot.state =
        super::begin_conflict_recovery(plan_conflict_recovery(&classify_conflicts(&text)));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn actor_refreshes_and_publishes_a_typed_conflict_snapshot() {
        let actor = GitConflictActor::new();
        actor.refresh().await.expect("refresh");
        let snapshot = actor.snapshot();
        assert!(snapshot
            .state
            .plan
            .paths
            .iter()
            .all(|path| !path.is_empty()));
        assert!(snapshot.state.selected_path.is_none());
    }

    #[tokio::test]
    async fn actor_rejects_selection_that_is_not_in_the_snapshot() {
        let actor = GitConflictActor::new();
        actor.refresh().await.expect("refresh");
        assert!(actor.select_path("not-a-conflict").await.is_err());
        assert!(actor.snapshot().state.selected_path.is_none());
    }
}
