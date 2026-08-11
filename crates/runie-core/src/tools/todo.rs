//! Replayable todo snapshots. The caller owns the list; each write replaces it.

use crate::task_owner::{spawn_actor_worker, TaskOwner};
use crate::types::{AgentTool, AgentToolResult, ToolResultContent};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: TodoStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TodoSnapshot {
    pub items: Vec<TodoItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoPlanStatus {
    Empty,
    Pending,
    InProgress,
    Complete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct TodoPlanSummary {
    pub status: TodoPlanStatus,
    pub completed: usize,
    pub pending: usize,
    pub in_progress: usize,
}

impl TodoPlanSummary {
    pub fn terminal_line(&self) -> String {
        format!(
            "Todo plan: {:?} · completed={} pending={} in_progress={}",
            self.status, self.completed, self.pending, self.in_progress
        )
    }
}

pub fn summarize_todo_plan(snapshot: &TodoSnapshot) -> TodoPlanSummary {
    let completed = snapshot
        .items
        .iter()
        .filter(|item| item.status == TodoStatus::Completed)
        .count();
    let in_progress = snapshot
        .items
        .iter()
        .filter(|item| item.status == TodoStatus::InProgress)
        .count();
    let pending = snapshot.items.len() - completed - in_progress;
    let status = if snapshot.items.is_empty() {
        TodoPlanStatus::Empty
    } else if completed == snapshot.items.len() {
        TodoPlanStatus::Complete
    } else if in_progress > 0 {
        TodoPlanStatus::InProgress
    } else {
        TodoPlanStatus::Pending
    };
    TodoPlanSummary {
        status,
        completed,
        pending,
        in_progress,
    }
}

enum TodoMessage {
    Replace {
        snapshot: TodoSnapshot,
        reply: oneshot::Sender<Result<TodoSnapshot, String>>,
    },
}

#[derive(Clone)]
pub struct TodoActor {
    tx: mpsc::Sender<TodoMessage>,
    snapshot: watch::Receiver<TodoSnapshot>,
    shared_snapshot: watch::Receiver<crate::SharedSnapshot<TodoSnapshot>>,
    _owner: Arc<TaskOwner>,
}

impl Default for TodoActor {
    fn default() -> Self {
        let initial = TodoSnapshot { items: Vec::new() };
        let (snapshot_tx, snapshot) = watch::channel(initial.clone());
        let (shared_tx, shared_snapshot) = watch::channel(crate::SharedSnapshot::new(initial));
        let (tx, owner) =
            spawn_actor_worker!(32, move |mut rx: mpsc::Receiver<TodoMessage>| async move {
                while let Some(TodoMessage::Replace { snapshot, reply }) = rx.recv().await {
                    crate::publish_shared_snapshot(&snapshot_tx, &shared_tx, snapshot.clone());
                    let _ = reply.send(Ok(snapshot));
                }
            });
        Self {
            tx,
            snapshot,
            shared_snapshot,
            _owner: owner,
        }
    }
}

impl TodoActor {
    pub async fn replace(&self, snapshot: TodoSnapshot) -> Result<TodoSnapshot, String> {
        validate_snapshot(&snapshot)?;
        let (reply, result) = oneshot::channel();
        self.tx
            .send(TodoMessage::Replace { snapshot, reply })
            .await
            .map_err(|_| "todo actor is closed".to_owned())?;
        result
            .await
            .map_err(|_| "todo actor dropped the result".to_owned())?
    }
    pub fn snapshot(&self) -> TodoSnapshot {
        self.snapshot.borrow().clone()
    }

    pub fn shared_snapshot(&self) -> crate::SharedSnapshot<TodoSnapshot> {
        self.shared_snapshot.borrow().clone()
    }

    pub fn shared_subscribe(&self) -> watch::Receiver<crate::SharedSnapshot<TodoSnapshot>> {
        self.shared_snapshot.clone()
    }

    pub fn summary(&self) -> TodoPlanSummary {
        summarize_todo_plan(&self.snapshot())
    }
}

impl TodoSnapshot {
    pub fn terminal_lines(&self) -> Vec<String> {
        self.items
            .iter()
            .map(|item| format!("{} · {:?} · {}", item.id, item.status, item.content))
            .collect()
    }
}

fn validate_snapshot(snapshot: &TodoSnapshot) -> Result<(), String> {
    if snapshot.items.len() > 100 {
        return Err("at most 100 todo items are allowed".into());
    }
    if snapshot
        .items
        .iter()
        .any(|item| item.id.trim().is_empty() || item.content.trim().is_empty())
    {
        return Err("todo ids and content must not be empty".into());
    }
    let mut ids = HashSet::with_capacity(snapshot.items.len());
    if snapshot.items.iter().any(|item| !ids.insert(&item.id)) {
        return Err("todo ids must be unique".into());
    }
    if snapshot
        .items
        .iter()
        .filter(|item| item.status == TodoStatus::InProgress)
        .count()
        > 1
    {
        return Err("only one todo may be in progress".into());
    }
    Ok(())
}

#[derive(Default)]
pub struct TodoWriteTool;

pub(crate) fn result(value: serde_json::Value) -> AgentToolResult {
    AgentToolResult {
        content: vec![ToolResultContent::Text {
            text: "todos updated".into(),
        }],
        details: value,
        ..Default::default()
    }
}

pub(crate) async fn execute_write(
    call: &crate::types::ToolCall,
    ctx: &crate::tools::executor::ToolExecContext,
) -> Result<AgentToolResult, String> {
    let Some(hook) = &ctx.hooks.todo_write else {
        return Err("todo_write requires an owning todo hook".into());
    };
    let snapshot = serde_json::from_value(call.arguments.clone())
        .map_err(|error| format!("invalid todo snapshot: {error}"))?;
    Ok(result(hook(snapshot).await?))
}

#[async_trait::async_trait]
impl AgentTool for TodoWriteTool {
    fn name(&self) -> &str {
        "todo_write"
    }
    fn label(&self) -> &str {
        "Update todos"
    }
    fn description(&self) -> &str {
        "Replace the replayable task list with the supplied items."
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": { "items": { "type": "array", "maxItems": 100 } },
            "required": ["items"]
        }))
    }
    fn validate_arguments(&self, args: &serde_json::Value) -> Result<(), String> {
        let snapshot: TodoSnapshot = serde_json::from_value(args.clone())
            .map_err(|error| format!("invalid todo snapshot: {error}"))?;
        validate_snapshot(&snapshot)
    }
    async fn execute(
        &self,
        _tool_call_id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let snapshot: TodoSnapshot = serde_json::from_value(args)
            .map_err(|error| format!("invalid todo snapshot: {error}"))?;
        let details = serde_json::to_value(&snapshot).map_err(|error| error.to_string())?;
        Ok(AgentToolResult {
            content: vec![ToolResultContent::Text {
                text: "todos updated".into(),
            }],
            details,
            ..AgentToolResult::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_snapshot_is_validated_as_one_data_value() {
        let tool = TodoWriteTool;
        let valid = serde_json::json!({"items": [
            {"id":"a", "content":"inspect", "status":"completed"},
            {"id":"b", "content":"implement", "status":"in_progress"}
        ]});
        assert!(tool.validate_arguments(&valid).is_ok());
        let duplicate_active = serde_json::json!({"items": [
            {"id":"a", "content":"one", "status":"in_progress"},
            {"id":"b", "content":"two", "status":"in_progress"}
        ]});
        assert!(tool.validate_arguments(&duplicate_active).is_err());
        let duplicate_id = serde_json::json!({"items": [
            {"id":"same", "content":"one", "status":"pending"},
            {"id":"same", "content":"two", "status":"completed"}
        ]});
        assert!(tool
            .validate_arguments(&duplicate_id)
            .unwrap_err()
            .contains("unique"));
    }

    #[test]
    fn todo_plan_summary_is_a_replayable_status_projection() {
        let snapshot = TodoSnapshot {
            items: vec![
                TodoItem {
                    id: "done".into(),
                    content: "ship".into(),
                    status: TodoStatus::Completed,
                },
                TodoItem {
                    id: "next".into(),
                    content: "verify".into(),
                    status: TodoStatus::InProgress,
                },
            ],
        };
        let summary = summarize_todo_plan(&snapshot);
        assert_eq!(summary.status, TodoPlanStatus::InProgress);
        assert_eq!(
            (summary.completed, summary.in_progress, summary.pending),
            (1, 1, 0)
        );
        assert_eq!(
            summary.terminal_line(),
            "Todo plan: InProgress · completed=1 pending=0 in_progress=1"
        );
    }

    #[tokio::test]
    async fn todo_actor_replaces_one_owned_snapshot() {
        let actor = TodoActor::default();
        let snapshot = TodoSnapshot {
            items: vec![TodoItem {
                id: "a".into(),
                content: "ship".into(),
                status: TodoStatus::InProgress,
            }],
        };
        assert_eq!(actor.replace(snapshot.clone()).await.unwrap(), snapshot);
        assert_eq!(actor.snapshot(), snapshot);
        assert_eq!(actor.shared_snapshot().get(), &snapshot);
        assert_eq!(actor.shared_snapshot().strong_count(), 2);
        assert_eq!(actor.summary().status, TodoPlanStatus::InProgress);
        assert_eq!(
            actor.snapshot().terminal_lines(),
            vec!["a · InProgress · ship"]
        );
    }

    #[tokio::test]
    async fn todo_actor_rejects_invalid_state_before_emitting_an_event() {
        let actor = TodoActor::default();
        let error = actor
            .replace(TodoSnapshot {
                items: vec![TodoItem {
                    id: "".into(),
                    content: "missing id".into(),
                    status: TodoStatus::Pending,
                }],
            })
            .await
            .unwrap_err();
        assert!(error.contains("must not be empty"));
        assert!(actor.snapshot().items.is_empty());
    }
}
