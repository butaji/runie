//! Actor-owned session journal projection.
//!
//! This is the first persistence seam for Pi-compatible session behavior:
//! message entries are appended from the typed event bus, never by the TUI or
//! provider adapters. Storage backends can consume the immutable snapshot
//! later without becoming a second state owner.

use std::{collections::HashMap, sync::Arc};

use tokio::sync::{mpsc, oneshot, watch};

use crate::events::EventBus;
use crate::task_owner::{mailbox_ack, spawn_actor_worker, spawn_owned_worker, TaskOwner};
use crate::types::{AgentEvent, AgentMessage, ThinkingLevel};

/// Pi session configuration changes which are journal facts but not
/// `AgentMessage` values. They are kept separate from the message projection
/// until the complete JSONL record union is migrated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionConfigRecord {
    ModelChanged {
        provider: String,
        model_id: String,
    },
    ThinkingLevelChanged {
        level: ThinkingLevel,
    },
    ActiveToolsChanged {
        tool_names: Vec<String>,
    },
    BranchSummaryCreated {
        from_id: String,
        summary: String,
        details: Option<serde_json::Value>,
    },
    CustomSessionEntryCreated {
        custom_type: String,
        data: Option<serde_json::Value>,
    },
    CompactionCreated {
        summary: String,
        retained_tail: Vec<AgentMessage>,
        tokens_before: u64,
        details: Option<serde_json::Value>,
        usage: Option<crate::types::Usage>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionConfigEntry {
    pub id: String,
    pub seq: u64,
    pub parent_id: Option<String>,
    pub timestamp: i64,
    pub record: SessionConfigRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEntry {
    pub id: String,
    pub seq: u64,
    pub parent_id: Option<String>,
    pub timestamp: i64,
    pub message: AgentMessage,
    /// Pi's message entry may terminate the current run without changing the
    /// message payload. Keep this journal fact outside `AgentMessage` so the
    /// wire message union remains unchanged.
    pub terminate: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub sequence: u64,
    pub leaf_id: Option<String>,
    pub entries: Vec<SessionEntry>,
    /// Ordered configuration records delivered through the session actor.
    /// Message `entries` remains the compatibility projection for existing
    /// callers; these records never become synthetic messages.
    pub config_records: Vec<SessionConfigEntry>,
}

impl SessionSnapshot {
    /// Parse the message-only subset emitted by [`Self::to_jsonl`].
    /// Validation follows Pi's v4 invariants for header, sequence, and parent
    /// linkage; unsupported mutation kinds are rejected explicitly.
    #[allow(
        clippy::too_many_lines,
        reason = "the importer keeps the JSONL validation boundary explicit"
    )]
    pub fn from_jsonl(input: &str) -> Result<(String, String, Self), String> {
        let mut lines = input.lines().filter(|line| !line.trim().is_empty());
        let header: serde_json::Value = serde_json::from_str(
            lines
                .next()
                .ok_or_else(|| "session JSONL is empty".to_owned())?,
        )
        .map_err(|error| format!("invalid session header: {error}"))?;
        if header.get("kind").and_then(serde_json::Value::as_str) != Some("header")
            || header.get("version").and_then(serde_json::Value::as_u64) != Some(4)
        {
            return Err("unsupported session header (expected JSONL v4)".into());
        }
        let session_id = header
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "session header is missing id".to_owned())?
            .to_owned();
        let cwd = header
            .get("cwd")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "session header is missing cwd".to_owned())?
            .to_owned();
        let mut snapshot = Self::default();
        for (line_index, line) in lines.enumerate() {
            let value: serde_json::Value = serde_json::from_str(line)
                .map_err(|error| format!("invalid session entry {}: {error}", line_index + 2))?;
            if value.get("kind").and_then(serde_json::Value::as_str) != Some("entry")
                || value.get("lane").and_then(serde_json::Value::as_str) != Some("main")
            {
                return Err(format!(
                    "unsupported session mutation at line {}",
                    line_index + 2
                ));
            }
            let seq = value
                .get("seq")
                .and_then(serde_json::Value::as_u64)
                .ok_or_else(|| format!("session entry {} is missing seq", line_index + 2))?;
            if seq != snapshot.sequence + 1 {
                return Err(format!(
                    "session entry {} has non-consecutive seq",
                    line_index + 2
                ));
            }
            let parent_id = match value.get("parentId") {
                Some(value) if value.is_null() => None,
                Some(value) => Some(
                    value
                        .as_str()
                        .ok_or_else(|| {
                            format!("session entry {} has invalid parentId", line_index + 2)
                        })?
                        .to_owned(),
                ),
                None => {
                    return Err(format!(
                        "session entry {} is missing parentId",
                        line_index + 2
                    ))
                }
            };
            if parent_id != snapshot.leaf_id {
                return Err(format!(
                    "session entry {} has broken parent link",
                    line_index + 2
                ));
            }
            let id = value
                .get("id")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| format!("session entry {} is missing id", line_index + 2))?
                .to_owned();
            let timestamp = value
                .get("timestamp")
                .and_then(serde_json::Value::as_i64)
                .ok_or_else(|| format!("session entry {} has invalid timestamp", line_index + 2))?;
            let entry_type = value
                .get("type")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| format!("session entry {} is missing type", line_index + 2))?;
            if entry_type != "message" {
                let record = match entry_type {
                    "model_change" => SessionConfigRecord::ModelChanged {
                        provider: value
                            .get("provider")
                            .and_then(serde_json::Value::as_str)
                            .ok_or_else(|| {
                                format!("session entry {} is missing provider", line_index + 2)
                            })?
                            .to_owned(),
                        model_id: value
                            .get("modelId")
                            .and_then(serde_json::Value::as_str)
                            .ok_or_else(|| {
                                format!("session entry {} is missing modelId", line_index + 2)
                            })?
                            .to_owned(),
                    },
                    "thinking_level_change" => SessionConfigRecord::ThinkingLevelChanged {
                        level: serde_json::from_value(
                            value.get("thinkingLevel").cloned().ok_or_else(|| {
                                format!("session entry {} is missing thinkingLevel", line_index + 2)
                            })?,
                        )
                        .map_err(|error| format!("invalid thinkingLevel: {error}"))?,
                    },
                    "active_tools_change" => SessionConfigRecord::ActiveToolsChanged {
                        tool_names: serde_json::from_value(
                            value.get("activeToolNames").cloned().ok_or_else(|| {
                                format!(
                                    "session entry {} is missing activeToolNames",
                                    line_index + 2
                                )
                            })?,
                        )
                        .map_err(|error| format!("invalid activeToolNames: {error}"))?,
                    },
                    "branch_summary" => SessionConfigRecord::BranchSummaryCreated {
                        from_id: value
                            .get("fromId")
                            .and_then(serde_json::Value::as_str)
                            .ok_or_else(|| {
                                format!("session entry {} is missing fromId", line_index + 2)
                            })?
                            .to_owned(),
                        summary: value
                            .get("summary")
                            .and_then(serde_json::Value::as_str)
                            .ok_or_else(|| {
                                format!("session entry {} is missing summary", line_index + 2)
                            })?
                            .to_owned(),
                        details: value
                            .get("details")
                            .cloned()
                            .filter(|value| !value.is_null()),
                    },
                    "custom" => SessionConfigRecord::CustomSessionEntryCreated {
                        custom_type: value
                            .get("customType")
                            .and_then(serde_json::Value::as_str)
                            .ok_or_else(|| {
                                format!("session entry {} is missing customType", line_index + 2)
                            })?
                            .to_owned(),
                        data: value.get("data").cloned().filter(|value| !value.is_null()),
                    },
                    "compaction" => SessionConfigRecord::CompactionCreated {
                        summary: value
                            .get("summary")
                            .and_then(serde_json::Value::as_str)
                            .ok_or_else(|| {
                                format!("session entry {} is missing summary", line_index + 2)
                            })?
                            .to_owned(),
                        retained_tail: serde_json::from_value(
                            value.get("retainedTail").cloned().ok_or_else(|| {
                                format!("session entry {} is missing retainedTail", line_index + 2)
                            })?,
                        )
                        .map_err(|error| format!("invalid retainedTail: {error}"))?,
                        tokens_before: value
                            .get("tokensBefore")
                            .and_then(serde_json::Value::as_u64)
                            .ok_or_else(|| {
                                format!("session entry {} is missing tokensBefore", line_index + 2)
                            })?,
                        details: value
                            .get("details")
                            .cloned()
                            .filter(|value| !value.is_null()),
                        usage: value
                            .get("usage")
                            .cloned()
                            .filter(|value| !value.is_null())
                            .map(serde_json::from_value)
                            .transpose()
                            .map_err(|error| format!("invalid usage: {error}"))?,
                    },
                    _ => {
                        return Err(format!(
                            "unsupported session mutation at line {}",
                            line_index + 2
                        ))
                    }
                };
                snapshot.sequence = seq;
                snapshot.leaf_id = Some(id.clone());
                snapshot.config_records.push(SessionConfigEntry {
                    id,
                    seq,
                    parent_id,
                    timestamp,
                    record,
                });
                continue;
            }
            let message =
                serde_json::from_value(value.get("message").cloned().ok_or_else(|| {
                    format!("session entry {} is missing message", line_index + 2)
                })?)
                .map_err(|error| {
                    format!(
                        "session entry {} has invalid message: {error}",
                        line_index + 2
                    )
                })?;
            let terminate = value
                .get("terminate")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            snapshot.sequence = seq;
            snapshot.leaf_id = Some(id.clone());
            snapshot.entries.push(SessionEntry {
                id,
                seq,
                parent_id,
                timestamp,
                message,
                terminate,
            });
        }
        Ok((session_id, cwd, snapshot))
    }

    /// Encode the message lane using Pi's JSONL v4 header/entry shape.
    /// Filesystem writes stay outside this pure projection function.
    #[allow(
        clippy::too_many_lines,
        reason = "JSONL encoding keeps the Pi record union explicit"
    )]
    pub fn to_jsonl(&self, session_id: &str, created_at: i64, cwd: &str) -> String {
        let mut lines = Vec::with_capacity(self.entries.len() + 1);
        lines.push(
            serde_json::json!({
                "kind": "header",
                "version": 4,
                "id": session_id,
                "createdAt": created_at,
                "cwd": cwd,
            })
            .to_string(),
        );
        let mut entry_lines = self
            .entries
            .iter()
            .map(|session_entry| {
                let mut entry = serde_json::json!({
                    "kind": "entry",
                    "lane": "main",
                    "type": "message",
                    "id": session_entry.id,
                    "parentId": session_entry.parent_id,
                    "seq": session_entry.seq,
                    "timestamp": session_entry.timestamp,
                    "message": session_entry.message,
                });
                if session_entry.terminate {
                    entry["terminate"] = serde_json::Value::Bool(true);
                }
                entry.to_string()
            })
            .collect::<Vec<_>>();
        entry_lines.extend(self.config_records.iter().map(|session_entry| {
            let (entry_type, mut entry) = match &session_entry.record {
                SessionConfigRecord::ModelChanged { provider, model_id } => (
                    "model_change",
                    serde_json::json!({
                        "provider": provider,
                        "modelId": model_id,
                    }),
                ),
                SessionConfigRecord::ThinkingLevelChanged { level } => (
                    "thinking_level_change",
                    serde_json::json!({
                        "thinkingLevel": level,
                    }),
                ),
                SessionConfigRecord::ActiveToolsChanged { tool_names } => (
                    "active_tools_change",
                    serde_json::json!({ "activeToolNames": tool_names }),
                ),
                SessionConfigRecord::BranchSummaryCreated {
                    from_id,
                    summary,
                    details,
                } => (
                    "branch_summary",
                    serde_json::json!({ "fromId": from_id, "summary": summary, "details": details }),
                ),
                SessionConfigRecord::CustomSessionEntryCreated { custom_type, data } => (
                    "custom",
                    serde_json::json!({ "customType": custom_type, "data": data }),
                ),
                SessionConfigRecord::CompactionCreated {
                    summary,
                    retained_tail,
                    tokens_before,
                    details,
                    usage,
                } => (
                    "compaction",
                    serde_json::json!({
                        "summary": summary,
                        "retainedTail": retained_tail,
                        "tokensBefore": tokens_before,
                        "details": details,
                        "usage": usage,
                    }),
                ),
            };
            entry["kind"] = serde_json::Value::String("entry".into());
            entry["lane"] = serde_json::Value::String("main".into());
            entry["type"] = serde_json::Value::String(entry_type.into());
            entry["id"] = serde_json::Value::String(session_entry.id.clone());
            entry["parentId"] = session_entry
                .parent_id
                .clone()
                .map_or(serde_json::Value::Null, serde_json::Value::String);
            entry["seq"] = serde_json::Value::Number(session_entry.seq.into());
            entry["timestamp"] = serde_json::Value::Number(session_entry.timestamp.into());
            entry.to_string()
        }));
        entry_lines.sort_by_key(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .ok()
                .and_then(|value| value.get("seq").and_then(serde_json::Value::as_u64))
                .unwrap_or_default()
        });
        lines.extend(entry_lines);
        format!("{}\n", lines.join("\n"))
    }
}

enum Command {
    Append(Box<AgentMessage>, bool, oneshot::Sender<()>),
    Config(SessionConfigRecord, oneshot::Sender<()>),
    Import(SessionSnapshot, oneshot::Sender<()>),
    Reset(oneshot::Sender<()>),
    Flush(oneshot::Sender<()>),
}

#[derive(Clone)]
pub struct SessionActor {
    tx: mpsc::Sender<Command>,
    snapshot: watch::Receiver<SessionSnapshot>,
    _owner: Arc<TaskOwner>,
    _bus_owner: Option<Arc<TaskOwner>>,
}

impl SessionActor {
    #[allow(
        clippy::too_many_lines,
        reason = "the actor constructor keeps its complete mailbox reduction loop visible"
    )]
    pub fn new() -> Self {
        let (snapshot_tx, snapshot) = watch::channel(SessionSnapshot::default());
        let (tx, owner) = spawn_actor_worker!(32, |mut rx: mpsc::Receiver<Command>| async move {
            let mut state = SessionSnapshot::default();
            let mut next_id = 1_u64;
            while let Some(command) = rx.recv().await {
                match command {
                    Command::Append(message, terminate, reply) => {
                        state.sequence += 1;
                        let id = format!("entry-{}", next_id);
                        next_id += 1;
                        let entry = SessionEntry {
                            id: id.clone(),
                            seq: state.sequence,
                            parent_id: state.leaf_id.clone(),
                            timestamp: message.timestamp(),
                            message: *message,
                            terminate,
                        };
                        state.leaf_id = Some(id);
                        state.entries.push(entry);
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(());
                    }
                    Command::Config(record, reply) => {
                        state.sequence += 1;
                        let id = format!("entry-{}", next_id);
                        next_id += 1;
                        let entry = SessionConfigEntry {
                            id: id.clone(),
                            seq: state.sequence,
                            parent_id: state.leaf_id.clone(),
                            // Configuration events carry no Pi timestamp;
                            // the journal uses a deterministic zero until a
                            // source timestamp is added to the event.
                            timestamp: 0,
                            record,
                        };
                        state.leaf_id = Some(id);
                        state.config_records.push(entry);
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(());
                    }
                    Command::Import(imported, reply) => {
                        next_id = imported
                            .entries
                            .iter()
                            .filter_map(|entry| entry.id.strip_prefix("entry-"))
                            .filter_map(|value| value.parse::<u64>().ok())
                            .max()
                            .unwrap_or(imported.sequence)
                            .saturating_add(1);
                        state = imported;
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(());
                    }
                    Command::Reset(reply) => {
                        state = SessionSnapshot::default();
                        next_id = 1;
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(());
                    }
                    Command::Flush(reply) => {
                        let _ = reply.send(());
                    }
                }
            }
        });
        Self {
            tx,
            snapshot,
            _owner: owner,
            _bus_owner: None,
        }
    }

    #[allow(
        clippy::too_many_lines,
        reason = "the session bus bridge keeps event-to-record ownership explicit"
    )]
    pub fn new_with_bus(bus: &EventBus) -> Self {
        let mut actor = Self::new();
        let events = bus.subscribe();
        let tx = actor.tx.clone();
        actor._bus_owner = Some(spawn_owned_worker!(async move {
            let mut events = events;
            let mut tool_termination = HashMap::<String, bool>::new();
            while let Ok(event) = events.recv().await {
                match event {
                    AgentEvent::MessageEnd { message } => {
                        let terminate = match &message {
                            AgentMessage::ToolResult(result) => tool_termination
                                .remove(&result.tool_call_id)
                                .unwrap_or(false),
                            _ => false,
                        };
                        if !mailbox_ack!(tx, |reply| {
                            Command::Append(Box::new(message), terminate, reply)
                        }) {
                            break;
                        }
                    }
                    AgentEvent::ModelChanged { model } => {
                        if !mailbox_ack!(tx, |reply| {
                            Command::Config(
                                SessionConfigRecord::ModelChanged {
                                    provider: model.provider,
                                    model_id: model.id,
                                },
                                reply,
                            )
                        }) {
                            break;
                        }
                    }
                    AgentEvent::ThinkingLevelChanged { level } => {
                        if !mailbox_ack!(tx, |reply| {
                            Command::Config(
                                SessionConfigRecord::ThinkingLevelChanged { level },
                                reply,
                            )
                        }) {
                            break;
                        }
                    }
                    AgentEvent::ActiveToolsChanged { tool_names } => {
                        if !mailbox_ack!(tx, |reply| {
                            Command::Config(
                                SessionConfigRecord::ActiveToolsChanged { tool_names },
                                reply,
                            )
                        }) {
                            break;
                        }
                    }
                    AgentEvent::BranchSummaryCreated {
                        from_id,
                        summary,
                        details,
                    } => {
                        if !mailbox_ack!(tx, |reply| {
                            Command::Config(
                                SessionConfigRecord::BranchSummaryCreated {
                                    from_id,
                                    summary,
                                    details,
                                },
                                reply,
                            )
                        }) {
                            break;
                        }
                    }
                    AgentEvent::CustomSessionEntryCreated { custom_type, data } => {
                        if !mailbox_ack!(tx, |reply| {
                            Command::Config(
                                SessionConfigRecord::CustomSessionEntryCreated {
                                    custom_type,
                                    data,
                                },
                                reply,
                            )
                        }) {
                            break;
                        }
                    }
                    AgentEvent::CompactionCreated {
                        summary,
                        retained_tail,
                        tokens_before,
                        details,
                        usage,
                    } => {
                        if !mailbox_ack!(tx, |reply| {
                            Command::Config(
                                SessionConfigRecord::CompactionCreated {
                                    summary,
                                    retained_tail,
                                    tokens_before,
                                    details,
                                    usage,
                                },
                                reply,
                            )
                        }) {
                            break;
                        }
                    }
                    AgentEvent::ToolExecutionEnd {
                        tool_call_id,
                        result,
                        ..
                    } => {
                        let terminate = result
                            .get("terminate")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false);
                        tool_termination.insert(tool_call_id, terminate);
                    }
                    AgentEvent::Reset if !mailbox_ack!(tx, Command::Reset) => break,
                    AgentEvent::Reset => {}
                    _ => {}
                }
            }
        }));
        actor
    }

    pub async fn append(&self, message: AgentMessage) {
        let _ = mailbox_ack!(self.tx, |reply| {
            Command::Append(Box::new(message), false, reply)
        });
    }

    /// Apply a session configuration fact through the owning mailbox.
    pub async fn record_config(&self, record: SessionConfigRecord) {
        let _ = mailbox_ack!(self.tx, |reply| Command::Config(record, reply));
    }

    /// Apply session-owned configuration facts from a replay event sequence.
    /// The reducer remains the actor boundary; callers do not mutate the
    /// snapshot or manufacture message entries.
    #[allow(
        clippy::too_many_lines,
        reason = "session event dispatch keeps each journal variant explicit"
    )]
    pub async fn apply_event(&self, event: &AgentEvent) {
        match event {
            AgentEvent::ModelChanged { model } => {
                self.record_config(SessionConfigRecord::ModelChanged {
                    provider: model.provider.clone(),
                    model_id: model.id.clone(),
                })
                .await;
            }
            AgentEvent::ThinkingLevelChanged { level } => {
                self.record_config(SessionConfigRecord::ThinkingLevelChanged { level: *level })
                    .await;
            }
            AgentEvent::ActiveToolsChanged { tool_names } => {
                self.record_config(SessionConfigRecord::ActiveToolsChanged {
                    tool_names: tool_names.clone(),
                })
                .await;
            }
            AgentEvent::BranchSummaryCreated {
                from_id,
                summary,
                details,
            } => {
                self.record_config(SessionConfigRecord::BranchSummaryCreated {
                    from_id: from_id.clone(),
                    summary: summary.clone(),
                    details: details.clone(),
                })
                .await;
            }
            AgentEvent::CustomSessionEntryCreated { custom_type, data } => {
                self.record_config(SessionConfigRecord::CustomSessionEntryCreated {
                    custom_type: custom_type.clone(),
                    data: data.clone(),
                })
                .await;
            }
            AgentEvent::CompactionCreated {
                summary,
                retained_tail,
                tokens_before,
                details,
                usage,
            } => {
                self.record_config(SessionConfigRecord::CompactionCreated {
                    summary: summary.clone(),
                    retained_tail: retained_tail.clone(),
                    tokens_before: *tokens_before,
                    details: details.clone(),
                    usage: usage.clone(),
                })
                .await;
            }
            AgentEvent::Reset => self.reset().await,
            _ => {}
        }
    }

    pub async fn reset(&self) {
        let _ = mailbox_ack!(self.tx, Command::Reset);
    }

    /// Restore a validated Pi JSONL message lane through the actor mailbox.
    /// Parsing is pure; replacing the owned journal and publishing its
    /// snapshot are performed only by the actor worker.
    pub async fn restore_jsonl(&self, input: &str) -> Result<(String, String), String> {
        let (session_id, cwd, snapshot) = SessionSnapshot::from_jsonl(input)?;
        if !mailbox_ack!(self.tx, |reply| Command::Import(snapshot, reply)) {
            return Err("session actor restore was not acknowledged".to_owned());
        }
        Ok((session_id, cwd))
    }

    pub fn snapshot(&self) -> SessionSnapshot {
        self.snapshot.borrow().clone()
    }

    pub async fn flush(&self) {
        let _ = mailbox_ack!(self.tx, Command::Flush);
    }
}

impl Default for SessionActor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventBus;
    use crate::types::{ToolResultContent, ToolResultMessage, UserContent, UserMessage};

    fn user(text: &str) -> AgentMessage {
        AgentMessage::User(UserMessage {
            content: vec![UserContent::Text { text: text.into() }],
            timestamp: 7,
        })
    }

    #[tokio::test]
    async fn actor_reduces_ordered_entries_and_parent_links() {
        let actor = SessionActor::new();
        actor.append(user("one")).await;
        actor.append(user("two")).await;
        let snapshot = actor.snapshot();
        assert_eq!(snapshot.sequence, 2);
        assert_eq!(snapshot.entries[0].parent_id, None);
        assert_eq!(snapshot.entries[1].parent_id.as_deref(), Some("entry-1"));
        assert_eq!(snapshot.leaf_id.as_deref(), Some("entry-2"));
    }

    #[tokio::test]
    async fn bus_message_end_and_reset_are_the_only_projection_inputs() {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        bus.publish(AgentEvent::MessageEnd {
            message: user("one"),
        });
        tokio::task::yield_now().await;
        assert_eq!(actor.snapshot().entries.len(), 1);
        bus.publish(AgentEvent::Reset);
        tokio::task::yield_now().await;
        assert!(actor.snapshot().entries.is_empty());
    }

    #[tokio::test]
    async fn bus_configuration_events_reduce_to_ordered_session_records() {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        bus.publish(AgentEvent::ModelChanged {
            model: crate::types::Model {
                id: "model-1".into(),
                provider: "provider-1".into(),
                ..Default::default()
            },
        });
        bus.publish(AgentEvent::ThinkingLevelChanged {
            level: crate::types::ThinkingLevel::High,
        });
        actor.flush().await;
        let records = actor.snapshot().config_records;
        assert_eq!(records.len(), 2);
        assert!(matches!(
            records[0].record,
            SessionConfigRecord::ModelChanged { ref provider, ref model_id }
                if provider == "provider-1" && model_id == "model-1"
        ));
        assert!(matches!(
            records[1].record,
            SessionConfigRecord::ThinkingLevelChanged {
                level: crate::types::ThinkingLevel::High
            }
        ));
        assert_eq!(records[0].seq, 1);
        assert_eq!(records[1].parent_id.as_deref(), Some("entry-1"));
    }

    #[tokio::test]
    async fn bus_tool_termination_is_attached_to_the_owned_session_entry() {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        bus.publish(AgentEvent::ToolExecutionEnd {
            tool_call_id: "call-1".into(),
            tool_name: "stop".into(),
            result: serde_json::json!({"terminate": true}),
            is_error: false,
        });
        bus.publish(AgentEvent::MessageEnd {
            message: AgentMessage::ToolResult(ToolResultMessage {
                tool_call_id: "call-1".into(),
                tool_name: "stop".into(),
                content: vec![ToolResultContent::Text {
                    text: "done".into(),
                }],
                ..Default::default()
            }),
        });
        actor.flush().await;
        assert!(actor.snapshot().entries[0].terminate);
    }

    #[tokio::test]
    async fn snapshot_exports_pi_jsonl_v4_header_and_parented_message_entry() {
        let actor = SessionActor::new();
        actor.append(user("one")).await;
        actor.append(user("two")).await;
        let lines = actor.snapshot().to_jsonl("session-1", 5, "/workspace");
        let values = lines
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("jsonl line"))
            .collect::<Vec<_>>();
        assert_eq!(values[0]["kind"], "header");
        assert_eq!(values[0]["version"], 4);
        assert_eq!(values[1]["type"], "message");
        assert_eq!(values[1]["parentId"], serde_json::Value::Null);
        assert_eq!(values[2]["parentId"], "entry-1");
        assert_eq!(values[2]["seq"], 2);
    }

    #[test]
    fn jsonl_round_trip_preserves_pi_terminate_entry_metadata() {
        let snapshot = SessionSnapshot {
            sequence: 1,
            leaf_id: Some("entry-1".into()),
            entries: vec![SessionEntry {
                id: "entry-1".into(),
                seq: 1,
                parent_id: None,
                timestamp: 7,
                message: user("stop here"),
                terminate: true,
            }],
            config_records: Vec::new(),
        };
        let jsonl = snapshot.to_jsonl("session-1", 5, "/workspace");
        assert!(jsonl.contains("\"terminate\":true"));
        let (_, _, imported) = SessionSnapshot::from_jsonl(&jsonl).expect("valid JSONL");
        assert!(imported.entries[0].terminate);
    }

    #[test]
    fn jsonl_round_trip_preserves_configuration_records() {
        let snapshot = SessionSnapshot {
            sequence: 2,
            leaf_id: Some("entry-2".into()),
            entries: Vec::new(),
            config_records: vec![
                SessionConfigEntry {
                    id: "entry-1".into(),
                    seq: 1,
                    parent_id: None,
                    timestamp: 0,
                    record: SessionConfigRecord::ThinkingLevelChanged {
                        level: crate::types::ThinkingLevel::High,
                    },
                },
                SessionConfigEntry {
                    id: "entry-2".into(),
                    seq: 2,
                    parent_id: Some("entry-1".into()),
                    timestamp: 0,
                    record: SessionConfigRecord::ActiveToolsChanged {
                        tool_names: vec!["read".into(), "bash".into()],
                    },
                },
            ],
        };
        let jsonl = snapshot.to_jsonl("session-1", 5, "/workspace");
        assert!(jsonl.contains("\"type\":\"thinking_level_change\""));
        let (_, _, imported) = SessionSnapshot::from_jsonl(&jsonl).expect("valid JSONL");
        assert_eq!(imported.config_records, snapshot.config_records);
    }

    #[tokio::test]
    async fn snapshot_jsonl_round_trips_through_validated_importer() {
        let actor = SessionActor::new();
        actor.append(user("one")).await;
        actor.append(user("two")).await;
        let original = actor.snapshot();
        let jsonl = original.to_jsonl("session-1", 5, "/workspace");

        let (session_id, cwd, imported) = SessionSnapshot::from_jsonl(&jsonl).expect("valid JSONL");
        assert_eq!(session_id, "session-1");
        assert_eq!(cwd, "/workspace");
        assert_eq!(imported, original);
    }

    #[tokio::test]
    async fn actor_restores_jsonl_and_continues_owned_entry_ids() {
        let source = SessionActor::new();
        source.append(user("one")).await;
        source.append(user("two")).await;
        let jsonl = source.snapshot().to_jsonl("session-1", 5, "/workspace");

        let restored = SessionActor::new();
        assert_eq!(
            restored.restore_jsonl(&jsonl).await.expect("restore"),
            ("session-1".to_owned(), "/workspace".to_owned())
        );
        restored.append(user("three")).await;
        let snapshot = restored.snapshot();
        assert_eq!(snapshot.sequence, 3);
        assert_eq!(snapshot.entries[2].id, "entry-3");
        assert_eq!(snapshot.entries[2].parent_id.as_deref(), Some("entry-2"));
    }

    #[test]
    fn jsonl_import_rejects_broken_sequence_parent_and_entry_kind() {
        let header = serde_json::json!({
            "kind": "header", "version": 4, "id": "s", "createdAt": 5, "cwd": "/w"
        })
        .to_string();
        let message = serde_json::json!({
            "role": "user", "content": [{"type": "text", "text": "one"}], "timestamp": 7
        });
        let entry = |seq, parent, kind| {
            serde_json::json!({
                "kind": kind, "lane": "main", "type": "message", "id": "entry-1",
                "parentId": parent, "seq": seq, "timestamp": 7, "message": message
            })
            .to_string()
        };

        let broken_sequence = format!("{header}\n{}\n", entry(2, serde_json::Value::Null, "entry"));
        assert!(SessionSnapshot::from_jsonl(&broken_sequence).is_err());
        let broken_parent = format!(
            "{header}\n{}\n",
            entry(1, serde_json::json!("wrong"), "entry")
        );
        assert!(SessionSnapshot::from_jsonl(&broken_parent).is_err());
        let unsupported_kind = format!(
            "{header}\n{}\n",
            entry(1, serde_json::Value::Null, "branch")
        );
        assert!(SessionSnapshot::from_jsonl(&unsupported_kind).is_err());
    }
}
