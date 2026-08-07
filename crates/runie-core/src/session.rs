//! Actor-owned session journal projection.
//!
//! This is the first persistence seam for Pi-compatible session behavior:
//! message entries are appended from the typed event bus, never by the TUI or
//! provider adapters. Storage backends can consume the immutable snapshot
//! later without becoming a second state owner.

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

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
    OperationRecordCreated {
        record_type: String,
        data: serde_json::Value,
    },
}

/// Pure declarative mapping from application events to session-journal facts.
/// Both the bus bridge and replay path use this one table before sending the
/// fact through `SessionActor`'s mailbox.
macro_rules! session_config_record {
    ($event:expr) => {{
        match $event {
            AgentEvent::ModelChanged { model } => Some(SessionConfigRecord::ModelChanged {
                provider: model.provider.clone(),
                model_id: model.id.clone(),
            }),
            AgentEvent::ThinkingLevelChanged { level } => {
                Some(SessionConfigRecord::ThinkingLevelChanged { level: *level })
            }
            AgentEvent::ActiveToolsChanged { tool_names } => {
                Some(SessionConfigRecord::ActiveToolsChanged {
                    tool_names: tool_names.clone(),
                })
            }
            AgentEvent::BranchSummaryCreated {
                from_id,
                summary,
                details,
            } => Some(SessionConfigRecord::BranchSummaryCreated {
                from_id: from_id.clone(),
                summary: summary.clone(),
                details: details.clone(),
            }),
            AgentEvent::CustomSessionEntryCreated { custom_type, data } => {
                Some(SessionConfigRecord::CustomSessionEntryCreated {
                    custom_type: custom_type.clone(),
                    data: data.clone(),
                })
            }
            AgentEvent::CompactionCreated {
                summary,
                retained_tail,
                tokens_before,
                details,
                usage,
            } => Some(SessionConfigRecord::CompactionCreated {
                summary: summary.clone(),
                retained_tail: retained_tail.clone(),
                tokens_before: *tokens_before,
                details: details.clone(),
                usage: usage.clone(),
            }),
            AgentEvent::OperationRecordCreated { record_type, data } => {
                Some(SessionConfigRecord::OperationRecordCreated {
                    record_type: record_type.clone(),
                    data: data.clone(),
                })
            }
            _ => None,
        }
    }};
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
    /// Ordered, admitted Pi operation-lane records. Invalid or duplicate
    /// records never enter this projection.
    pub lane_records: Vec<SessionLaneRecordSnapshot>,
    /// Reducer-owned operation lifecycle projection keyed by Pi operation ID.
    /// Values are `started` or `aborted`; finished operations are removed.
    pub active_operations: BTreeMap<String, String>,
    /// Terminal Pi outcomes keyed by operation ID. This remains separate from
    /// active operations so completion is observable without retaining a
    /// finished operation in the active projection.
    pub operation_outcomes: BTreeMap<String, String>,
    /// Pi operation intent kinds keyed by operation ID.
    pub operation_kinds: BTreeMap<String, String>,
    /// Pi failure metadata keyed by operation ID.
    pub operation_errors: BTreeMap<String, OperationErrorSnapshot>,
    /// Last admitted Pi navigation intent. This is deliberately a projection
    /// only; branch context reconstruction remains owned by the session tree.
    pub navigation: Option<NavigationSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationSnapshot {
    pub target_id: Option<String>,
    pub summarize: bool,
    pub summary_entry_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationValidation {
    pub target_exists: bool,
    pub summary_exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationErrorSnapshot {
    pub code: String,
    pub message: String,
}

/// Lossless actor-owned projection of an admitted Pi operation-lane record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLaneRecordSnapshot {
    pub record_type: String,
    pub id: String,
    pub lane: Option<String>,
    pub seq: Option<u64>,
    pub timestamp: Option<i64>,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactionCutPoint {
    pub first_kept_entry_index: usize,
    pub turn_start_index: Option<usize>,
    pub is_split_turn: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionPreparation {
    pub history_indices: Vec<usize>,
    pub turn_prefix_indices: Vec<usize>,
    pub retained_indices: Vec<usize>,
    pub tokens_before: u64,
    pub cut_point: CompactionCutPoint,
}

/// Build the pure payload for Pi's async compaction owner. Only index
/// selection happens here; summary generation and journal publication remain
/// separate event-driven operations.
pub fn prepare_compaction_entries(
    entries: &[SessionEntry],
    token_estimates: &[u64],
    keep_recent_tokens: u64,
) -> Result<Option<CompactionPreparation>, String> {
    if entries.is_empty() {
        return Ok(None);
    }
    let cut_point = find_compaction_cut_point(
        entries,
        token_estimates,
        0,
        entries.len(),
        keep_recent_tokens,
    )?;
    let history_end = cut_point
        .turn_start_index
        .unwrap_or(cut_point.first_kept_entry_index);
    Ok(Some(CompactionPreparation {
        history_indices: (0..history_end).collect(),
        turn_prefix_indices: cut_point
            .turn_start_index
            .map(|start| (start..cut_point.first_kept_entry_index).collect())
            .unwrap_or_default(),
        retained_indices: (cut_point.first_kept_entry_index..entries.len()).collect(),
        tokens_before: token_estimates.iter().sum(),
        cut_point,
    }))
}

/// Select Pi's recent-context cut point without performing summarization.
/// `token_estimates` is supplied by the caller so estimation policy stays
/// explicit and testable; entries that cannot begin a turn (tool results) are
/// never selected as a cut point.
#[allow(
    clippy::too_many_lines,
    reason = "the Pi cut-point decision table stays explicit"
)]
pub fn find_compaction_cut_point(
    entries: &[SessionEntry],
    token_estimates: &[u64],
    start_index: usize,
    end_index: usize,
    keep_recent_tokens: u64,
) -> Result<CompactionCutPoint, String> {
    if start_index > end_index
        || end_index > entries.len()
        || entries.len() != token_estimates.len()
    {
        return Err("compaction cut-point bounds do not match entries".into());
    }
    let cut_points = (start_index..end_index)
        .filter(|index| {
            matches!(
                &entries[*index].message,
                AgentMessage::User(_) | AgentMessage::Assistant(_)
            )
        })
        .collect::<Vec<_>>();
    let Some(mut cut_index) = cut_points.first().copied() else {
        return Ok(CompactionCutPoint {
            first_kept_entry_index: start_index,
            turn_start_index: None,
            is_split_turn: false,
        });
    };
    let mut accumulated = 0;
    for index in (start_index..end_index).rev() {
        if !matches!(&entries[index].message, AgentMessage::ToolResult(_)) {
            accumulated += token_estimates[index];
        }
        if accumulated >= keep_recent_tokens {
            if let Some(candidate) = cut_points
                .iter()
                .copied()
                .find(|candidate| *candidate >= index)
            {
                cut_index = candidate;
            }
            break;
        }
    }
    let is_user = matches!(&entries[cut_index].message, AgentMessage::User(_));
    let turn_start_index = if is_user {
        None
    } else {
        (start_index..=cut_index)
            .rev()
            .find(|index| matches!(&entries[*index].message, AgentMessage::User(_)))
    };
    Ok(CompactionCutPoint {
        first_kept_entry_index: cut_index,
        turn_start_index,
        is_split_turn: turn_start_index.is_some(),
    })
}

/// Pi's durable operation-lane record families. The payload remains JSON at
/// the wire boundary, but classification is typed before the actor reducer
/// changes its owned projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionLaneRecordKind {
    OperationStarted,
    AbortRequested,
    OperationFinished,
    StepAttempt,
    ToolStarted,
    QueueEnqueued,
    QueueCancelled,
    WriteDeferred,
    Usage,
}

pub fn session_lane_record_kind(record_type: &str) -> Option<SessionLaneRecordKind> {
    Some(match record_type {
        "operation_started" => SessionLaneRecordKind::OperationStarted,
        "abort_requested" => SessionLaneRecordKind::AbortRequested,
        "operation_finished" => SessionLaneRecordKind::OperationFinished,
        "step_attempt" => SessionLaneRecordKind::StepAttempt,
        "tool_started" => SessionLaneRecordKind::ToolStarted,
        "queue_enqueued" => SessionLaneRecordKind::QueueEnqueued,
        "queue_cancelled" => SessionLaneRecordKind::QueueCancelled,
        "write_deferred" => SessionLaneRecordKind::WriteDeferred,
        "usage" => SessionLaneRecordKind::Usage,
        _ => return None,
    })
}

/// Validate the identity/admission rules that can be checked without IO.
/// Storage-specific sequence and lane checks remain owned by the future
/// durable storage actor.
pub fn validate_session_lane_record(
    snapshot: &SessionSnapshot,
    record_type: &str,
    data: &serde_json::Value,
) -> Result<SessionLaneRecordKind, String> {
    let kind = session_lane_record_kind(record_type)
        .ok_or_else(|| format!("unknown session lane record type {record_type:?}"))?;
    validate_session_lane_metadata(record_type, data)?;
    let operation_id = data
        .get("runId")
        .or_else(|| data.get("id"))
        .or_else(|| data.get("entryId"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty());
    if operation_id.is_none() {
        return Err(format!(
            "session lane record {record_type:?} is missing id/runId"
        ));
    }
    if kind == SessionLaneRecordKind::OperationStarted
        && operation_id.is_some_and(|id| snapshot.active_operations.contains_key(id))
    {
        return Err(format!(
            "session operation {:?} is already open",
            operation_id.expect("checked above")
        ));
    }
    Ok(kind)
}

/// Validate Pi's storage metadata when a wire record carries it. Compatibility
/// events may carry only a lane because they are created before persistence;
/// once sequence or timestamp metadata is present, the complete storage tuple
/// is required.
pub fn validate_session_lane_metadata(
    record_type: &str,
    data: &serde_json::Value,
) -> Result<(), String> {
    let has_metadata = ["seq", "timestamp"]
        .iter()
        .any(|field| data.get(*field).is_some());
    if !has_metadata {
        return Ok(());
    }
    let lane = data
        .get("lane")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("session lane record {record_type:?} has invalid lane"))?;
    let seq = data
        .get("seq")
        .and_then(serde_json::Value::as_u64)
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("session lane record {record_type:?} has invalid seq"))?;
    let timestamp = data
        .get("timestamp")
        .and_then(serde_json::Value::as_i64)
        .filter(|value| *value >= 0)
        .ok_or_else(|| format!("session lane record {record_type:?} has invalid timestamp"))?;
    let _ = (lane, seq, timestamp);
    Ok(())
}

/// Reduce one Pi operation record into the session-owned lifecycle projection.
/// Live event delivery and JSONL replay must use this same pure mapping so the
/// two paths cannot drift.
#[allow(
    clippy::too_many_lines,
    reason = "the Pi operation record table keeps live and replay projection rules together"
)]
fn reduce_operation_record(
    snapshot: &mut SessionSnapshot,
    record_type: &str,
    data: &serde_json::Value,
) {
    if validate_session_lane_record(snapshot, record_type, data).is_err() {
        return;
    }
    let Some(record_id) = data
        .get("runId")
        .or_else(|| data.get("id"))
        .or_else(|| data.get("entryId"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    snapshot.lane_records.push(SessionLaneRecordSnapshot {
        record_type: record_type.to_owned(),
        id: record_id.to_owned(),
        lane: data
            .get("lane")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        seq: data.get("seq").and_then(serde_json::Value::as_u64),
        timestamp: data.get("timestamp").and_then(serde_json::Value::as_i64),
        data: data.clone(),
    });
    if record_type == "operation_started"
        && data
            .get("intent")
            .and_then(|intent| intent.get("kind"))
            .and_then(serde_json::Value::as_str)
            == Some("navigation")
    {
        if let Some(intent) = data.get("intent") {
            snapshot.navigation = Some(NavigationSnapshot {
                target_id: intent
                    .get("targetId")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned),
                summarize: intent
                    .get("summarize")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                summary_entry_id: intent
                    .get("summaryEntryId")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned),
            });
        }
    }
    let Some(operation_id) = data
        .get("runId")
        .or_else(|| data.get("id"))
        .and_then(serde_json::Value::as_str)
    else {
        return;
    };
    match record_type {
        "operation_started" => {
            if let Some(kind) = data
                .get("intent")
                .and_then(|intent| intent.get("kind"))
                .and_then(serde_json::Value::as_str)
            {
                snapshot
                    .operation_kinds
                    .insert(operation_id.to_owned(), kind.to_owned());
            }
            snapshot
                .active_operations
                .insert(operation_id.to_owned(), "started".into());
        }
        "abort_requested" => {
            snapshot
                .active_operations
                .insert(operation_id.to_owned(), "aborted".into());
        }
        "operation_finished" => {
            snapshot.active_operations.remove(operation_id);
            if let Some(outcome) = data.get("outcome").and_then(serde_json::Value::as_str) {
                snapshot
                    .operation_outcomes
                    .insert(operation_id.to_owned(), outcome.to_owned());
                if let Some(error) = data.get("error") {
                    if let (Some(code), Some(message)) = (
                        error.get("code").and_then(serde_json::Value::as_str),
                        error.get("message").and_then(serde_json::Value::as_str),
                    ) {
                        snapshot.operation_errors.insert(
                            operation_id.to_owned(),
                            OperationErrorSnapshot {
                                code: code.to_owned(),
                                message: message.to_owned(),
                            },
                        );
                    }
                }
            }
        }
        _ => {}
    }
}

impl SessionSnapshot {
    /// Return the selected branch from oldest to newest journal node.
    /// Message and configuration records share the same parent/id namespace.
    pub fn branch_entry_ids(&self) -> Vec<String> {
        let mut parents = BTreeMap::new();
        for entry in &self.entries {
            parents.insert(entry.id.clone(), entry.parent_id.clone());
        }
        for entry in &self.config_records {
            parents.insert(entry.id.clone(), entry.parent_id.clone());
        }
        let mut path = Vec::new();
        let mut current = self.leaf_id.clone();
        while let Some(id) = current {
            if path.iter().any(|seen| seen == &id) {
                break;
            }
            current = parents.get(&id).cloned().flatten();
            path.push(id);
        }
        path.reverse();
        path
    }

    /// Create the message-lane fork prefix Pi would publish into a new
    /// session. The returned snapshot owns new sequence numbers while
    /// retaining the original parent/id graph; no actor or source snapshot
    /// is mutated.
    #[allow(
        clippy::too_many_lines,
        reason = "fork validation and projection stay one pure operation"
    )]
    pub fn fork_at_message(&self, target_id: &str) -> Result<Self, String> {
        if !self.entries.iter().any(|entry| entry.id == target_id) {
            return Err(format!("invalid fork target {target_id:?}"));
        }
        let branch = self.branch_entry_ids();
        if !branch.iter().any(|id| id == target_id) {
            return Err(format!(
                "fork target {target_id:?} is not on the selected branch"
            ));
        }
        let retained = branch
            .into_iter()
            .take_while(|id| id != target_id)
            .chain(std::iter::once(target_id.to_owned()))
            .collect::<std::collections::BTreeSet<_>>();
        let mut fork = Self {
            leaf_id: Some(target_id.to_owned()),
            ..Self::default()
        };
        let mut sequence = 0;
        for entry in &self.entries {
            if !retained.contains(&entry.id) {
                continue;
            }
            sequence += 1;
            let mut copy = entry.clone();
            copy.seq = sequence;
            fork.entries.push(copy);
        }
        for entry in &self.config_records {
            if !retained.contains(&entry.id) {
                continue;
            }
            sequence += 1;
            let mut copy = entry.clone();
            copy.seq = sequence;
            if let SessionConfigRecord::OperationRecordCreated { record_type, data } = &copy.record
            {
                reduce_operation_record(&mut fork, record_type, data);
            }
            fork.config_records.push(copy);
        }
        fork.sequence = sequence;
        Ok(fork)
    }

    /// Validate the currently projected navigation intent against journal IDs.
    /// This is pure and intentionally does not admit or mutate navigation.
    pub fn navigation_validation(&self) -> Option<NavigationValidation> {
        let ids = self
            .entries
            .iter()
            .map(|entry| entry.id.as_str())
            .chain(self.config_records.iter().map(|entry| entry.id.as_str()))
            .collect::<std::collections::BTreeSet<_>>();
        self.navigation
            .as_ref()
            .map(|navigation| NavigationValidation {
                target_exists: navigation
                    .target_id
                    .as_deref()
                    .is_some_and(|target| ids.contains(target)),
                summary_exists: navigation
                    .summary_entry_id
                    .as_deref()
                    .is_some_and(|summary| ids.contains(summary)),
            })
    }

    /// Parse the message-only subset emitted by [`Self::to_jsonl`].
    /// Validation follows Pi's v4 invariants for header, sequence, and parent
    /// linkage; unsupported mutation kinds are rejected explicitly.
    ///
    /// The filesystem actor should call [`Self::repair_jsonl_torn_tail`] before
    /// handing file contents to this parser. Keeping repair pure makes the
    /// recovery decision deterministic and leaves publication to the storage
    /// actor.
    #[allow(
        clippy::too_many_lines,
        clippy::cognitive_complexity,
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
                    "operation_started" | "operation_finished" | "abort_requested" => {
                        SessionConfigRecord::OperationRecordCreated {
                            record_type: entry_type.to_owned(),
                            data: value.clone(),
                        }
                    }
                    _ => {
                        return Err(format!(
                            "unsupported session mutation at line {}",
                            line_index + 2
                        ))
                    }
                };
                snapshot.sequence = seq;
                snapshot.leaf_id = Some(id.clone());
                reduce_operation_record(&mut snapshot, entry_type, &value);
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

    /// Repair the one failure Pi's JSONL loader may recover locally: an
    /// unterminated or invalid final physical line. A malformed non-final
    /// line is never discarded. Valid final content is only normalized by
    /// appending its missing newline; no mutation is interpreted here.
    pub fn repair_jsonl_torn_tail(input: &str) -> Result<String, String> {
        let mut lines = input.split('\n').collect::<Vec<_>>();
        while lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }
        let header = lines
            .first()
            .ok_or_else(|| "session JSONL is empty".to_owned())?;
        serde_json::from_str::<serde_json::Value>(header)
            .map_err(|error| format!("invalid session header: {error}"))?;
        for (index, line) in lines.iter().enumerate().skip(1) {
            if serde_json::from_str::<serde_json::Value>(line).is_err() {
                if index + 1 != lines.len() {
                    return Err(format!("invalid session entry {}", index + 1));
                }
                lines.truncate(index);
                return Ok(format!("{}\n", lines.join("\n")));
            }
        }
        Ok(format!("{}\n", lines.join("\n")))
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
                SessionConfigRecord::OperationRecordCreated { record_type, data } => (
                    record_type.as_str(),
                    data.clone(),
                ),
            };
            if matches!(
                entry_type,
                "operation_started" | "operation_finished" | "abort_requested"
            ) {
                if let Some(operation_id) = entry
                    .get("id")
                    .cloned()
                    .filter(|value| value.is_string())
                {
                    entry
                        .as_object_mut()
                        .expect("operation record is an object")
                        .entry("runId")
                        .or_insert(operation_id);
                }
            }
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
    PrepareCompaction {
        token_estimates: Vec<u64>,
        keep_recent_tokens: u64,
        reply: oneshot::Sender<Result<Option<CompactionPreparation>, String>>,
    },
}

enum StorageCommand {
    Publish {
        path: String,
        contents: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
}

/// Actor-owned atomic JSONL publication. Serialization stays outside this
/// actor; no caller can observe a partially written destination file.
#[derive(Clone)]
pub struct SessionStorageActor {
    tx: mpsc::Sender<StorageCommand>,
    _owner: Arc<TaskOwner>,
}

impl SessionStorageActor {
    pub fn new() -> Self {
        let (tx, owner) =
            spawn_actor_worker!(8, |mut rx: mpsc::Receiver<StorageCommand>| async move {
                while let Some(command) = rx.recv().await {
                    match command {
                        StorageCommand::Publish {
                            path,
                            contents,
                            reply,
                        } => {
                            let temporary = format!("{path}.tmp");
                            let result = async {
                                tokio::fs::write(&temporary, contents)
                                    .await
                                    .map_err(|error| format!("stage session JSONL: {error}"))?;
                                if let Err(error) = tokio::fs::rename(&temporary, &path).await {
                                    let _ = tokio::fs::remove_file(&temporary).await;
                                    return Err(format!("publish session JSONL: {error}"));
                                }
                                Ok(())
                            }
                            .await;
                            let _ = reply.send(result);
                        }
                    }
                }
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
}

impl Default for SessionStorageActor {
    fn default() -> Self {
        Self::new()
    }
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
                        if let SessionConfigRecord::OperationRecordCreated { record_type, data } =
                            state
                                .config_records
                                .last()
                                .expect("record was just inserted")
                                .record
                                .clone()
                        {
                            reduce_operation_record(&mut state, &record_type, &data);
                        }
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
                    Command::PrepareCompaction {
                        token_estimates,
                        keep_recent_tokens,
                        reply,
                    } => {
                        let _ = reply.send(prepare_compaction_entries(
                            &state.entries,
                            &token_estimates,
                            keep_recent_tokens,
                        ));
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
                    _ => {
                        if let Some(record) = session_config_record!(&event) {
                            if !mailbox_ack!(tx, |reply| Command::Config(record, reply)) {
                                break;
                            }
                        }
                    }
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
        if let Some(record) = session_config_record!(event) {
            self.record_config(record).await;
        } else if matches!(event, AgentEvent::Reset) {
            self.reset().await;
        }
    }

    pub async fn reset(&self) {
        let _ = mailbox_ack!(self.tx, Command::Reset);
    }

    /// Restore a validated Pi JSONL message lane through the actor mailbox.
    /// Parsing is pure; replacing the owned journal and publishing its
    /// snapshot are performed only by the actor worker.
    pub async fn restore_jsonl(&self, input: &str) -> Result<(String, String), String> {
        let repaired = SessionSnapshot::repair_jsonl_torn_tail(input)?;
        let (session_id, cwd, snapshot) = SessionSnapshot::from_jsonl(&repaired)?;
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

    /// Ask the session owner to prepare compaction from its current state.
    /// Callers provide deterministic token estimates; no snapshot mutation or
    /// summarization occurs in this command.
    pub async fn prepare_compaction(
        &self,
        token_estimates: Vec<u64>,
        keep_recent_tokens: u64,
    ) -> Result<Option<CompactionPreparation>, String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self
            .tx
            .send(Command::PrepareCompaction {
                token_estimates,
                keep_recent_tokens,
                reply: reply_tx,
            })
            .await
            .is_err()
        {
            return Err("session actor compaction request was not acknowledged".into());
        }
        reply_rx
            .await
            .map_err(|_| "session actor compaction response was dropped".to_owned())?
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
    async fn operation_records_reduce_to_owned_lifecycle_state() {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        bus.publish(AgentEvent::OperationRecordCreated {
            record_type: "operation_started".into(),
            data: serde_json::json!({"id": "op-1"}),
        });
        actor.flush().await;
        assert_eq!(actor.snapshot().active_operations["op-1"], "started");
        bus.publish(AgentEvent::OperationRecordCreated {
            record_type: "abort_requested".into(),
            data: serde_json::json!({"id": "op-1"}),
        });
        actor.flush().await;
        assert_eq!(actor.snapshot().active_operations["op-1"], "aborted");
        bus.publish(AgentEvent::OperationRecordCreated {
            record_type: "operation_finished".into(),
            data: serde_json::json!({"id": "op-1", "outcome": "aborted"}),
        });
        actor.flush().await;
        assert!(actor.snapshot().active_operations.is_empty());
        assert_eq!(actor.snapshot().operation_outcomes["op-1"], "aborted");
        assert_eq!(
            actor
                .snapshot()
                .lane_records
                .iter()
                .map(|record| record.record_type.as_str())
                .collect::<Vec<_>>(),
            vec!["operation_started", "abort_requested", "operation_finished"]
        );
        let original = actor.snapshot();
        let jsonl = original.to_jsonl("session-ops", 5, "/workspace");
        let (_, _, imported) = SessionSnapshot::from_jsonl(&jsonl).expect("operation JSONL");
        assert_eq!(imported.active_operations, original.active_operations);
        assert_eq!(imported.operation_outcomes, original.operation_outcomes);
    }

    #[test]
    fn session_lane_record_validation_classifies_pi_families() {
        assert_eq!(
            session_lane_record_kind("tool_started"),
            Some(SessionLaneRecordKind::ToolStarted)
        );
        assert_eq!(
            validate_session_lane_record(
                &SessionSnapshot::default(),
                "usage",
                &serde_json::json!({"entryId": "entry-1"})
            ),
            Ok(SessionLaneRecordKind::Usage)
        );
        assert!(validate_session_lane_record(
            &SessionSnapshot::default(),
            "unknown",
            &serde_json::json!({"id": "record-1"})
        )
        .is_err());
    }

    #[test]
    fn session_lane_metadata_requires_a_complete_positive_storage_tuple() {
        let valid = serde_json::json!({
            "id": "op-1", "lane": "main", "seq": 1, "timestamp": 7
        });
        assert!(validate_session_lane_metadata("operation_started", &valid).is_ok());
        assert!(validate_session_lane_metadata(
            "operation_started",
            &serde_json::json!({"id": "op-1", "lane": "main", "seq": 1})
        )
        .is_err());
        assert!(validate_session_lane_metadata(
            "operation_started",
            &serde_json::json!({
                "id": "op-1", "lane": "main", "seq": 0, "timestamp": 7
            })
        )
        .is_err());
    }

    #[test]
    fn duplicate_or_malformed_lane_records_are_rejected_purely() {
        let mut snapshot = SessionSnapshot::default();
        assert!(validate_session_lane_record(
            &snapshot,
            "operation_started",
            &serde_json::json!({"id": "op-1"})
        )
        .is_ok());
        snapshot
            .active_operations
            .insert("op-1".into(), "started".into());
        assert!(validate_session_lane_record(
            &snapshot,
            "operation_started",
            &serde_json::json!({"id": "op-1"})
        )
        .is_err());
        assert!(validate_session_lane_record(
            &snapshot,
            "operation_finished",
            &serde_json::json!({"outcome": "completed"})
        )
        .is_err());
    }

    #[tokio::test]
    async fn navigation_operation_reduces_to_owned_intent_projection() {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        bus.publish(AgentEvent::OperationRecordCreated {
            record_type: "operation_started".into(),
            data: serde_json::json!({
                "id": "navigation-1",
                "intent": {
                    "kind": "navigation",
                    "targetId": "entry-target",
                    "summarize": true,
                    "summaryEntryId": "summary-target"
                }
            }),
        });
        actor.flush().await;
        assert_eq!(
            actor.snapshot().navigation,
            Some(NavigationSnapshot {
                target_id: Some("entry-target".into()),
                summarize: true,
                summary_entry_id: Some("summary-target".into()),
            })
        );
    }

    #[test]
    fn branch_entry_ids_follow_shared_parent_links() {
        let mut snapshot = SessionSnapshot::default();
        let message_entry = |seq: u64, parent_id: Option<&str>, id: &str| SessionEntry {
            id: id.into(),
            seq,
            parent_id: parent_id.map(str::to_owned),
            timestamp: 0,
            message: user("test"),
            terminate: false,
        };
        snapshot.entries = vec![
            message_entry(1, None, "message-1"),
            message_entry(2, Some("message-1"), "message-2"),
        ];
        snapshot.config_records = vec![SessionConfigEntry {
            id: "config-3".into(),
            seq: 3,
            parent_id: Some("message-2".into()),
            timestamp: 0,
            record: SessionConfigRecord::CustomSessionEntryCreated {
                custom_type: "test".into(),
                data: None,
            },
        }];
        snapshot.leaf_id = Some("config-3".into());
        assert_eq!(
            snapshot.branch_entry_ids(),
            ["message-1", "message-2", "config-3"]
        );
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the branch fixture spells out the parent graph"
    )]
    fn fork_at_message_resequences_only_the_validated_branch_prefix() {
        let snapshot = SessionSnapshot {
            entries: vec![
                SessionEntry {
                    id: "message-1".into(),
                    seq: 1,
                    parent_id: None,
                    timestamp: 0,
                    message: user("one"),
                    terminate: false,
                },
                SessionEntry {
                    id: "message-2".into(),
                    seq: 2,
                    parent_id: Some("message-1".into()),
                    timestamp: 0,
                    message: user("two"),
                    terminate: false,
                },
                SessionEntry {
                    id: "message-3".into(),
                    seq: 3,
                    parent_id: Some("message-2".into()),
                    timestamp: 0,
                    message: user("three"),
                    terminate: false,
                },
            ],
            leaf_id: Some("message-3".into()),
            ..SessionSnapshot::default()
        };
        let fork = snapshot.fork_at_message("message-2").expect("fork");
        assert_eq!(fork.sequence, 2);
        assert_eq!(fork.leaf_id.as_deref(), Some("message-2"));
        assert_eq!(
            fork.entries
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            ["message-1", "message-2"]
        );
        assert_eq!(
            fork.entries
                .iter()
                .map(|entry| entry.seq)
                .collect::<Vec<_>>(),
            [1, 2]
        );
        assert!(snapshot.fork_at_message("missing").is_err());
    }

    #[test]
    fn compaction_cut_point_preserves_recent_budget_and_reports_split_turn() {
        let entries = vec![
            SessionEntry {
                id: "user-1".into(),
                seq: 1,
                parent_id: None,
                timestamp: 0,
                message: user("request"),
                terminate: false,
            },
            SessionEntry {
                id: "assistant-1".into(),
                seq: 2,
                parent_id: Some("user-1".into()),
                timestamp: 0,
                message: AgentMessage::Assistant(Default::default()),
                terminate: false,
            },
        ];
        let cut = find_compaction_cut_point(&entries, &[40, 40], 0, 2, 20).expect("cut point");
        assert_eq!(cut.first_kept_entry_index, 1);
        assert_eq!(cut.turn_start_index, Some(0));
        assert!(cut.is_split_turn);
    }

    #[test]
    fn compaction_preparation_partitions_history_prefix_and_tail() {
        let entries = vec![
            SessionEntry {
                id: "user-1".into(),
                seq: 1,
                parent_id: None,
                timestamp: 0,
                message: user("request"),
                terminate: false,
            },
            SessionEntry {
                id: "assistant-1".into(),
                seq: 2,
                parent_id: Some("user-1".into()),
                timestamp: 0,
                message: AgentMessage::Assistant(Default::default()),
                terminate: false,
            },
        ];
        let preparation = prepare_compaction_entries(&entries, &[40, 40], 20)
            .expect("preparation")
            .expect("non-empty");
        assert!(preparation.history_indices.is_empty());
        assert_eq!(preparation.turn_prefix_indices, vec![0]);
        assert_eq!(preparation.retained_indices, vec![1]);
        assert_eq!(preparation.tokens_before, 80);
    }

    #[tokio::test]
    async fn actor_owns_compaction_preparation_through_mailbox() {
        let actor = SessionActor::new();
        actor.append(user("request")).await;
        actor
            .append(AgentMessage::Assistant(Default::default()))
            .await;
        let preparation = actor
            .prepare_compaction(vec![40, 40], 20)
            .await
            .expect("actor response")
            .expect("non-empty preparation");
        assert_eq!(preparation.retained_indices, vec![1]);
        assert_eq!(actor.snapshot().entries.len(), 2);
    }

    #[tokio::test]
    async fn storage_actor_publishes_a_complete_jsonl_file_atomically() {
        let path = std::env::temp_dir().join(format!(
            "runie-session-{}-{}.jsonl",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let path_string = path.to_string_lossy().into_owned();
        let storage = SessionStorageActor::new();
        let snapshot = SessionSnapshot::default();
        storage
            .publish_snapshot(&path_string, &snapshot, "session-1", 1, "/tmp")
            .await
            .expect("publish");
        let contents = tokio::fs::read_to_string(&path).await.expect("read");
        let header: serde_json::Value =
            serde_json::from_str(contents.lines().next().expect("header line"))
                .expect("header JSON");
        assert_eq!(header["kind"], "header");
        assert_eq!(header["version"], 4);
        assert!(!tokio::fs::try_exists(format!("{path_string}.tmp"))
            .await
            .expect("temporary file check"));
        let _ = tokio::fs::remove_file(path).await;
    }

    #[test]
    fn navigation_validation_checks_target_and_summary_ids() {
        let mut snapshot = SessionSnapshot {
            navigation: Some(NavigationSnapshot {
                target_id: Some("entry-1".into()),
                summarize: true,
                summary_entry_id: Some("summary-1".into()),
            }),
            ..SessionSnapshot::default()
        };
        snapshot.entries.push(SessionEntry {
            id: "entry-1".into(),
            seq: 1,
            parent_id: None,
            timestamp: 0,
            message: user("target"),
            terminate: false,
        });
        snapshot.config_records.push(SessionConfigEntry {
            id: "summary-1".into(),
            seq: 2,
            parent_id: Some("entry-1".into()),
            timestamp: 0,
            record: SessionConfigRecord::BranchSummaryCreated {
                from_id: "entry-1".into(),
                summary: "summary".into(),
                details: None,
            },
        });
        assert_eq!(
            snapshot.navigation_validation(),
            Some(NavigationValidation {
                target_exists: true,
                summary_exists: true,
            })
        );
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
            lane_records: Vec::new(),
            active_operations: BTreeMap::new(),
            operation_outcomes: BTreeMap::new(),
            operation_kinds: BTreeMap::new(),
            operation_errors: BTreeMap::new(),
            navigation: None,
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
            lane_records: Vec::new(),
            active_operations: BTreeMap::new(),
            operation_outcomes: BTreeMap::new(),
            operation_kinds: BTreeMap::new(),
            operation_errors: BTreeMap::new(),
            navigation: None,
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

    #[test]
    fn jsonl_repair_discards_only_a_torn_final_line() {
        let input = concat!(
            "{\"kind\":\"header\",\"version\":4,\"id\":\"s\",\"createdAt\":1,\"cwd\":\"/tmp\"}\n",
            "{\"kind\":\"entry\",\"lane\":\"main\",\"seq\":1}\n",
            "{\"kind\":\"entry\",\"lane\":"
        );
        let repaired = SessionSnapshot::repair_jsonl_torn_tail(input).expect("repair");
        assert!(repaired.ends_with("\"seq\":1}\n"));
        assert_eq!(repaired.lines().count(), 2);
    }

    #[test]
    fn jsonl_repair_rejects_a_broken_non_final_line() {
        let input = concat!(
            "{\"kind\":\"header\",\"version\":4}\n",
            "{broken}\n",
            "{\"kind\":\"entry\"}"
        );
        assert!(SessionSnapshot::repair_jsonl_torn_tail(input).is_err());
    }
}
