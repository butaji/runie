//! Replayable todo snapshots. The caller owns the list; each write replaces it.

use crate::task_owner::{spawn_actor_worker, TaskOwner};
use crate::types::{AgentTool, AgentToolResult, ToolResultContent};
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
    _owner: Arc<TaskOwner>,
}

impl Default for TodoActor {
    fn default() -> Self {
        let (snapshot_tx, snapshot) = watch::channel(TodoSnapshot { items: Vec::new() });
        let (tx, owner) =
            spawn_actor_worker!(32, move |mut rx: mpsc::Receiver<TodoMessage>| async move {
                while let Some(TodoMessage::Replace { snapshot, reply }) = rx.recv().await {
                    let _ = snapshot_tx.send(snapshot.clone());
                    let _ = reply.send(Ok(snapshot));
                }
            });
        Self {
            tx,
            snapshot,
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
