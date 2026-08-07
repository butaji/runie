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
use crate::types::{AgentEvent, AgentMessage, StopReason, ThinkingLevel};

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
    LabelChanged {
        target_id: String,
        label: Option<String>,
    },
    NameChanged {
        name: String,
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
            AgentEvent::SessionLabelChanged { target_id, label } => {
                Some(SessionConfigRecord::LabelChanged {
                    target_id: target_id.clone(),
                    label: label.clone(),
                })
            }
            AgentEvent::SessionNameChanged { name } => {
                Some(SessionConfigRecord::NameChanged { name: name.clone() })
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
pub struct SessionLaneFact {
    pub seq: u64,
    pub lane: String,
    pub leaf_id: Option<String>,
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

/// One ordered Pi session entry returned by the declarative entry query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEntryRecord {
    Message(Box<SessionEntry>),
    Config(Box<SessionConfigEntry>),
}

impl SessionEntryRecord {
    fn seq(&self) -> u64 {
        match self {
            Self::Message(entry) => entry.seq,
            Self::Config(entry) => entry.seq,
        }
    }

    fn record_type(&self) -> &'static str {
        match self {
            Self::Message(_) => "message",
            Self::Config(entry) => match &entry.record {
                SessionConfigRecord::ModelChanged { .. } => "model_change",
                SessionConfigRecord::ThinkingLevelChanged { .. } => "thinking_level_change",
                SessionConfigRecord::ActiveToolsChanged { .. } => "active_tools_change",
                SessionConfigRecord::LabelChanged { .. } => "label",
                SessionConfigRecord::NameChanged { .. } => "session_name",
                SessionConfigRecord::BranchSummaryCreated { .. } => "branch_summary",
                SessionConfigRecord::CustomSessionEntryCreated { .. } => "custom",
                SessionConfigRecord::CompactionCreated { .. } => "compaction",
                SessionConfigRecord::OperationRecordCreated { .. } => "record",
            },
        }
    }
}

/// Declarative equivalent of Pi's `EntryQuery`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionEntryQuery {
    pub record_type: Option<String>,
    pub custom_type: Option<String>,
    pub after_seq: Option<u64>,
    pub newest_first: bool,
    pub limit: Option<usize>,
}

/// Declarative Pi branch query. `start` is required; callers cannot silently
/// fall back to the current leaf when asking for a specific branch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionBranchEntryQuery {
    pub start: String,
    pub stop_at_type: Option<String>,
    pub stop_at_id: Option<String>,
    pub record_type: Option<String>,
    pub custom_type: Option<String>,
    pub newest_first: bool,
    pub limit: Option<usize>,
}

/// Pi's durable session statistics projection.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionStats {
    pub message_count: u64,
    pub cached_tokens: u64,
    pub uncached_tokens: u64,
    pub total_tokens: u64,
    pub cost_total: f64,
}

/// Ordered durable items returned by the Pi-compatible session log query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionLogItem {
    Entry {
        seq: u64,
        entry: SessionEntryRecord,
    },
    Record {
        seq: u64,
        record: SessionLaneRecordSnapshot,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub sequence: u64,
    pub leaf_id: Option<String>,
    pub entries: Vec<SessionEntry>,
    /// Actor-owned lane identity for message entries. Kept as a side
    /// projection while legacy callers still use the lane-neutral entry type.
    pub entry_lanes: BTreeMap<String, String>,
    /// Ordered configuration records delivered through the session actor.
    /// Message `entries` remains the compatibility projection for existing
    /// callers; these records never become synthetic messages.
    pub config_records: Vec<SessionConfigEntry>,
    /// Ordered Pi session-tree lane mutations, distinct from operation lanes.
    pub lane_facts: Vec<SessionLaneFact>,
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

/// Declarative read boundary matching Pi's durable operation-lane query.
/// Filters are applied by the session owner to its immutable snapshot; the
/// query does not expose mutable journal state to callers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionLaneQuery {
    pub lane: Option<String>,
    pub record_type: Option<String>,
    pub run_id: Option<String>,
    pub operation_kind: Option<String>,
    pub after_seq: Option<u64>,
    pub newest_first: bool,
    pub limit: Option<usize>,
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

/// Provider-owned input for Pi-compatible asynchronous compaction summary
/// generation. Session indexing prepares this value; the provider must not
/// mutate session state or publish journal records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionSummaryRequest {
    pub history: Vec<AgentMessage>,
    pub turn_prefix: Vec<AgentMessage>,
    pub retained_tail: Vec<AgentMessage>,
    pub tokens_before: u64,
    pub previous_summary: Option<String>,
}

impl CompactionSummaryRequest {
    /// Materialize provider input from one actor-owned preparation and the
    /// immutable journal entries it indexes. Invalid indices are rejected
    /// before any provider capability is invoked.
    pub fn from_preparation(
        preparation: &CompactionPreparation,
        entries: &[SessionEntry],
        previous_summary: Option<String>,
    ) -> Result<Self, String> {
        fn select(
            entries: &[SessionEntry],
            indices: &[usize],
        ) -> Result<Vec<AgentMessage>, String> {
            indices
                .iter()
                .map(|index| {
                    entries
                        .get(*index)
                        .map(|entry| entry.message.clone())
                        .ok_or_else(|| format!("compaction index {index} is out of bounds"))
                })
                .collect()
        }

        Ok(Self {
            history: select(entries, &preparation.history_indices)?,
            turn_prefix: select(entries, &preparation.turn_prefix_indices)?,
            retained_tail: select(entries, &preparation.retained_indices)?,
            tokens_before: preparation.tokens_before,
            previous_summary,
        })
    }
}

/// Provider result consumed by the session/loop coordinator before it emits
/// the actor-owned `CompactionCreated` event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionSummary {
    pub summary: String,
    pub usage: Option<crate::types::Usage>,
    pub details: Option<serde_json::Value>,
}

/// Pi's automatic-compaction threshold settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompactionSettings {
    pub enabled: bool,
    pub reserve_tokens: u64,
    pub keep_recent_tokens: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextUsageEstimate {
    pub tokens: u64,
    pub usage_tokens: u64,
    pub trailing_tokens: u64,
    pub last_usage_index: Option<usize>,
}

/// Pi's conservative four-characters-per-token message estimate.
pub fn estimate_message_tokens(message: &AgentMessage) -> u64 {
    const ESTIMATED_IMAGE_CHARS: u64 = 4_800;
    let chars = match message {
        AgentMessage::User(message) => message
            .content
            .iter()
            .map(|content| match content {
                crate::types::UserContent::Text { text } => pi_text_units(text),
                crate::types::UserContent::Image { .. } => ESTIMATED_IMAGE_CHARS,
            })
            .sum(),
        AgentMessage::Assistant(message) => message
            .content
            .iter()
            .map(|content| match content {
                crate::types::AssistantContent::Text { text }
                | crate::types::AssistantContent::Thinking { text } => pi_text_units(text),
                crate::types::AssistantContent::ToolCall(call) => {
                    pi_text_units(&call.name)
                        + serde_json::to_string(&call.arguments)
                            .map(|value| pi_text_units(&value))
                            .unwrap_or_default()
                }
            })
            .sum(),
        AgentMessage::ToolResult(message) => message
            .content
            .iter()
            .map(|content| match content {
                crate::types::ToolResultContent::Text { text } => pi_text_units(text),
                crate::types::ToolResultContent::Image { .. } => ESTIMATED_IMAGE_CHARS,
            })
            .sum(),
        AgentMessage::CompactionSummary(message) => pi_text_units(&message.summary),
        AgentMessage::Custom(_) => 0,
    };
    chars.saturating_add(3) / 4
}

/// JavaScript's `String.length` counts UTF-16 code units, which is the unit
/// used by Pi's token heuristic rather than Rust's UTF-8 byte length.
fn pi_text_units(text: &str) -> u64 {
    text.encode_utf16().count() as u64
}

/// Estimate context tokens using the latest valid assistant usage and the
/// conservative estimate for messages after it, matching Pi's harness.
pub fn estimate_context_tokens(messages: &[AgentMessage]) -> ContextUsageEstimate {
    let last_usage_index = messages
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, message)| {
            let AgentMessage::Assistant(assistant) = message else {
                return None;
            };
            let usage_tokens = assistant_usage_tokens(assistant);
            (assistant.stop_reason != Some(StopReason::Aborted)
                && assistant.stop_reason != Some(StopReason::Error)
                && usage_tokens > 0)
                .then_some(index)
        });
    let Some(index) = last_usage_index else {
        let trailing_tokens = messages.iter().map(estimate_message_tokens).sum();
        return ContextUsageEstimate {
            tokens: trailing_tokens,
            usage_tokens: 0,
            trailing_tokens,
            last_usage_index: None,
        };
    };
    let usage_tokens = match &messages[index] {
        AgentMessage::Assistant(assistant) => assistant_usage_tokens(assistant),
        _ => 0,
    };
    let trailing_tokens = messages[index + 1..]
        .iter()
        .map(estimate_message_tokens)
        .sum();
    ContextUsageEstimate {
        tokens: usage_tokens + trailing_tokens,
        usage_tokens,
        trailing_tokens,
        last_usage_index: Some(index),
    }
}

fn assistant_usage_tokens(message: &crate::types::AssistantMessage) -> u64 {
    let usage = &message.usage;
    if usage.total_tokens > 0 {
        usage.total_tokens
    } else {
        usage.input + usage.output + usage.cache_read + usage.cache_write
    }
}

/// Return whether Pi's harness should begin automatic compaction.
///
/// The summarizer and publication remain asynchronous actor-owned operations;
/// this function only makes the source-backed threshold decision.
pub fn should_compact(
    context_tokens: u64,
    context_window: u64,
    settings: CompactionSettings,
) -> bool {
    settings.enabled && context_tokens > context_window.saturating_sub(settings.reserve_tokens)
}

/// Pure provider-context boundary after the newest Pi compaction record.
///
/// The summary remains journal metadata until the provider-specific message
/// projector materializes it. Retained messages are carried by the
/// compaction record; ordinary message indices identify only entries written
/// after that boundary, so callers cannot accidentally send the compacted
/// prefix again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompactionContextProjection {
    pub summary: String,
    pub tokens_before: u64,
    pub timestamp: i64,
    pub retained_tail: Vec<AgentMessage>,
    pub message_indices: Vec<usize>,
}

impl CompactionContextProjection {
    /// Materialize Pi's internal context-message sequence. The summary keeps
    /// its distinct role until `convert_to_llm` applies provider wire rules.
    pub fn messages(&self, entries: &[SessionEntry]) -> Vec<AgentMessage> {
        let mut messages =
            Vec::with_capacity(1 + self.retained_tail.len() + self.message_indices.len());
        messages.push(AgentMessage::CompactionSummary(
            crate::types::CompactionSummaryMessage {
                summary: self.summary.clone(),
                tokens_before: self.tokens_before,
                timestamp: self.timestamp,
            },
        ));
        messages.extend(self.retained_tail.clone());
        messages.extend(
            self.message_indices
                .iter()
                .filter_map(|index| entries.get(*index).map(|entry| entry.message.clone())),
        );
        messages
    }
}

impl SessionSnapshot {
    pub fn entry_lane(&self, entry_id: &str) -> Option<&str> {
        self.entry_lanes.get(entry_id).map(String::as_str)
    }

    /// Reduce ordered Pi lane mutations into the latest leaf per lane.
    pub fn lanes(&self) -> BTreeMap<String, Option<String>> {
        let mut changes = vec![(0_u64, "main".to_owned(), None)];
        for entry in &self.entries {
            let lane = self
                .entry_lanes
                .get(&entry.id)
                .cloned()
                .unwrap_or_else(|| "main".into());
            changes.push((entry.seq, lane, Some(entry.id.clone())));
        }
        changes.extend(
            self.lane_facts
                .iter()
                .map(|fact| (fact.seq, fact.lane.clone(), fact.leaf_id.clone())),
        );
        changes.sort_by_key(|(seq, _, _)| *seq);
        let mut lanes = BTreeMap::new();
        for (_, lane, leaf_id) in changes {
            lanes.insert(lane, leaf_id);
        }
        lanes
    }

    /// Reduce ordered Pi session-name facts to the latest name.
    pub fn name(&self) -> Option<String> {
        self.config_records.iter().rev().find_map(|entry| {
            if let SessionConfigRecord::NameChanged { name } = &entry.record {
                Some(name.clone())
            } else {
                None
            }
        })
    }

    /// Reduce ordered Pi label facts into the effective label map. This is a
    /// pure read projection; the session actor remains the only writer.
    pub fn labels(&self) -> BTreeMap<String, String> {
        let mut labels = BTreeMap::new();
        for entry in &self.config_records {
            if let SessionConfigRecord::LabelChanged { target_id, label } = &entry.record {
                if let Some(label) = label {
                    labels.insert(target_id.clone(), label.clone());
                } else {
                    labels.remove(target_id);
                }
            }
        }
        labels
    }

    /// Return the first entry selected by Pi's ordered entry query.
    pub fn find_entry(&self, query: &SessionEntryQuery) -> Option<SessionEntryRecord> {
        self.find_entries(query).into_iter().next()
    }

    /// Return the first entry selected by an explicit branch query.
    pub fn find_entry_on_branch(
        &self,
        query: &SessionBranchEntryQuery,
    ) -> Result<Option<SessionEntryRecord>, String> {
        self.find_entries_on_branch(query)
            .map(|entries| entries.into_iter().next())
    }

    /// Find entries on one validated parent-linked branch.
    #[allow(
        clippy::too_many_lines,
        reason = "the Pi branch query keeps validation, projection, and ordering together"
    )]
    pub fn find_entries_on_branch(
        &self,
        query: &SessionBranchEntryQuery,
    ) -> Result<Vec<SessionEntryRecord>, String> {
        let mut parents = BTreeMap::new();
        for entry in &self.entries {
            parents.insert(entry.id.clone(), entry.parent_id.clone());
        }
        for entry in &self.config_records {
            parents.insert(entry.id.clone(), entry.parent_id.clone());
        }
        if !parents.contains_key(&query.start) {
            return Err(format!("branch start {:?} was not found", query.start));
        }
        let mut ids = Vec::new();
        let mut current = Some(query.start.clone());
        while let Some(id) = current {
            if ids.iter().any(|seen| seen == &id) {
                return Err("branch contains a parent cycle".into());
            }
            ids.push(id.clone());
            let entry = self
                .find_entries(&SessionEntryQuery::default())
                .into_iter()
                .find(|entry| match entry {
                    SessionEntryRecord::Message(entry) => entry.id == id,
                    SessionEntryRecord::Config(entry) => entry.id == id,
                });
            let Some(entry) = entry else { break };
            let entry_type = entry.record_type();
            if query.stop_at_id.as_deref() == Some(id.as_str())
                || query.stop_at_type.as_deref() == Some(entry_type)
            {
                break;
            }
            current = parents.get(&id).cloned().flatten();
        }
        ids.reverse();
        let id_set = ids.into_iter().collect::<std::collections::HashSet<_>>();
        let mut entries = self
            .find_entries(&SessionEntryQuery {
                record_type: query.record_type.clone(),
                custom_type: query.custom_type.clone(),
                ..SessionEntryQuery::default()
            })
            .into_iter()
            .filter(|entry| match entry {
                SessionEntryRecord::Message(entry) => id_set.contains(&entry.id),
                SessionEntryRecord::Config(entry) => id_set.contains(&entry.id),
            })
            .collect::<Vec<_>>();
        if query.newest_first {
            entries.reverse();
        }
        if let Some(limit) = query.limit {
            entries.truncate(limit);
        }
        Ok(entries)
    }

    /// Return message/config entries and operation records in journal order.
    pub fn get_log(&self, after_seq: Option<u64>, limit: Option<usize>) -> Vec<SessionLogItem> {
        let mut items = self
            .find_entries(&SessionEntryQuery {
                after_seq,
                ..SessionEntryQuery::default()
            })
            .into_iter()
            .map(|entry| SessionLogItem::Entry {
                seq: entry.seq(),
                entry,
            })
            .chain(self.lane_records.iter().filter_map(|record| {
                let seq = record.seq?;
                (after_seq.is_none_or(|after| seq > after)).then(|| SessionLogItem::Record {
                    seq,
                    record: record.clone(),
                })
            }))
            .collect::<Vec<_>>();
        items.sort_by_key(|item| match item {
            SessionLogItem::Entry { seq, .. } | SessionLogItem::Record { seq, .. } => *seq,
        });
        if let Some(limit) = limit {
            items.truncate(limit);
        }
        items
    }

    /// Return unfinished operation starts newest-first, matching Pi's
    /// recovery query. The limit is applied after ordering.
    pub fn find_open_operations(
        &self,
        lane: &str,
        limit: Option<usize>,
    ) -> Vec<SessionLaneRecordSnapshot> {
        let mut records = self
            .lane_records
            .iter()
            .filter(|record| {
                record.record_type == "operation_started"
                    && record.lane.as_deref() == Some(lane)
                    && self.active_operations.contains_key(&record.id)
            })
            .cloned()
            .collect::<Vec<_>>();
        records.reverse();
        if let Some(limit) = limit {
            records.truncate(limit);
        }
        records
    }

    /// Recompute Pi's session statistics from the immutable journal.
    pub fn stats(&self) -> SessionStats {
        let mut stats = SessionStats {
            message_count: self.entries.len() as u64,
            ..SessionStats::default()
        };
        for record in &self.lane_records {
            if record.record_type != "usage" {
                continue;
            }
            let Some(usage) = record.data.get("usage") else {
                continue;
            };
            stats.cached_tokens += usage
                .get("cacheRead")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default();
            stats.uncached_tokens += usage
                .get("input")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default()
                + usage
                    .get("cacheWrite")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default();
            stats.total_tokens += usage
                .get("totalTokens")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default();
            stats.cost_total += usage
                .get("cost")
                .and_then(|cost| cost.get("total"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or_default();
        }
        stats
    }

    /// Find ordered message/config entries using Pi's EntryQuery semantics.
    pub fn find_entries(&self, query: &SessionEntryQuery) -> Vec<SessionEntryRecord> {
        let mut entries = self
            .entries
            .iter()
            .cloned()
            .map(|entry| SessionEntryRecord::Message(Box::new(entry)))
            .chain(
                self.config_records
                    .iter()
                    .cloned()
                    .map(|entry| SessionEntryRecord::Config(Box::new(entry))),
            )
            .filter(|entry| {
                query
                    .record_type
                    .as_deref()
                    .is_none_or(|record_type| entry.record_type() == record_type)
                    && query.after_seq.is_none_or(|after| entry.seq() > after)
                    && query.custom_type.as_deref().is_none_or(|custom_type| {
                        matches!(
                            entry,
                            SessionEntryRecord::Config(entry)
                                if matches!(
                                    &entry.record,
                                    SessionConfigRecord::CustomSessionEntryCreated {
                                        custom_type: value,
                                        ..
                                    } if value == custom_type
                                )
                        )
                    })
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(SessionEntryRecord::seq);
        if query.newest_first {
            entries.reverse();
        }
        if let Some(limit) = query.limit {
            entries.truncate(limit);
        }
        entries
    }

    /// Find admitted operation-lane records using Pi's ordered query rules.
    pub fn find_lane_records(&self, query: &SessionLaneQuery) -> Vec<SessionLaneRecordSnapshot> {
        let mut records = self
            .lane_records
            .iter()
            .filter(|record| {
                query
                    .lane
                    .as_deref()
                    .is_none_or(|lane| record.lane.as_deref() == Some(lane))
                    && query
                        .record_type
                        .as_deref()
                        .is_none_or(|record_type| record.record_type == record_type)
                    && query.run_id.as_deref().is_none_or(|run_id| {
                        record.data.get("runId").and_then(serde_json::Value::as_str) == Some(run_id)
                    })
                    && query.operation_kind.as_deref().is_none_or(|kind| {
                        record
                            .data
                            .get("intent")
                            .and_then(|intent| intent.get("kind"))
                            .and_then(serde_json::Value::as_str)
                            == Some(kind)
                    })
                    && query
                        .after_seq
                        .is_none_or(|after| record.seq.is_some_and(|seq| seq > after))
            })
            .cloned()
            .collect::<Vec<_>>();
        if query.newest_first {
            records.reverse();
        }
        if let Some(limit) = query.limit {
            records.truncate(limit);
        }
        records
    }

    /// Build the latest-compaction context boundary without mutating the
    /// actor-owned journal. Deferred assistant results are excluded because
    /// Pi's context builder does not send them to the provider.
    #[allow(
        clippy::too_many_lines,
        reason = "the projection keeps Pi's compaction boundary decision table explicit"
    )]
    pub fn compaction_context_projection(&self) -> Option<CompactionContextProjection> {
        let compaction = self
            .config_records
            .iter()
            .filter_map(|entry| match &entry.record {
                SessionConfigRecord::CompactionCreated {
                    summary,
                    retained_tail,
                    tokens_before,
                    ..
                } => Some((
                    entry.seq,
                    summary,
                    retained_tail,
                    *tokens_before,
                    entry.timestamp,
                )),
                _ => None,
            })
            .max_by_key(|(seq, ..)| *seq)?;
        let (_, summary, retained_tail, tokens_before, timestamp) = compaction;
        let message_indices = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.seq > compaction.0)
            .filter(|(_, entry)| {
                !matches!(
                    &entry.message,
                    AgentMessage::Assistant(message)
                        if message.stop_reason == Some(StopReason::Deferred)
                )
            })
            .map(|(index, _)| index)
            .collect();
        Some(CompactionContextProjection {
            summary: summary.clone(),
            tokens_before,
            timestamp,
            retained_tail: retained_tail.clone(),
            message_indices,
        })
    }
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

/// Typed internal representation of a Pi operation-lane fact. The payload is
/// deliberately retained losslessly because Pi may add fields without a
/// Runie release; only the family is closed over here.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionLaneRecord {
    OperationStarted(serde_json::Value),
    AbortRequested(serde_json::Value),
    OperationFinished(serde_json::Value),
    StepAttempt(serde_json::Value),
    ToolStarted(serde_json::Value),
    QueueEnqueued(serde_json::Value),
    QueueCancelled(serde_json::Value),
    WriteDeferred(serde_json::Value),
    Usage(serde_json::Value),
}

impl SessionLaneRecord {
    pub fn decode(record_type: &str, data: &serde_json::Value) -> Result<Self, String> {
        let payload = data.clone();
        match session_lane_record_kind(record_type) {
            Some(SessionLaneRecordKind::OperationStarted) => Ok(Self::OperationStarted(payload)),
            Some(SessionLaneRecordKind::AbortRequested) => Ok(Self::AbortRequested(payload)),
            Some(SessionLaneRecordKind::OperationFinished) => Ok(Self::OperationFinished(payload)),
            Some(SessionLaneRecordKind::StepAttempt) => Ok(Self::StepAttempt(payload)),
            Some(SessionLaneRecordKind::ToolStarted) => Ok(Self::ToolStarted(payload)),
            Some(SessionLaneRecordKind::QueueEnqueued) => Ok(Self::QueueEnqueued(payload)),
            Some(SessionLaneRecordKind::QueueCancelled) => Ok(Self::QueueCancelled(payload)),
            Some(SessionLaneRecordKind::WriteDeferred) => Ok(Self::WriteDeferred(payload)),
            Some(SessionLaneRecordKind::Usage) => Ok(Self::Usage(payload)),
            None => Err(format!("unknown session lane record type {record_type:?}")),
        }
    }

    pub fn kind(&self) -> SessionLaneRecordKind {
        match self {
            Self::OperationStarted(_) => SessionLaneRecordKind::OperationStarted,
            Self::AbortRequested(_) => SessionLaneRecordKind::AbortRequested,
            Self::OperationFinished(_) => SessionLaneRecordKind::OperationFinished,
            Self::StepAttempt(_) => SessionLaneRecordKind::StepAttempt,
            Self::ToolStarted(_) => SessionLaneRecordKind::ToolStarted,
            Self::QueueEnqueued(_) => SessionLaneRecordKind::QueueEnqueued,
            Self::QueueCancelled(_) => SessionLaneRecordKind::QueueCancelled,
            Self::WriteDeferred(_) => SessionLaneRecordKind::WriteDeferred,
            Self::Usage(_) => SessionLaneRecordKind::Usage,
        }
    }

    /// Return the Pi record identity through one typed lane-family boundary.
    ///
    /// Pi uses `id` for operation records, `runId` for operation-owned facts,
    /// and `entryId` for entry-owned facts. Callers must not duplicate this
    /// wire-shape table or guess an identity from arbitrary payload fields.
    pub fn identity(&self) -> Option<&str> {
        let payload = match self {
            Self::OperationStarted(payload)
            | Self::AbortRequested(payload)
            | Self::OperationFinished(payload)
            | Self::StepAttempt(payload)
            | Self::ToolStarted(payload)
            | Self::QueueEnqueued(payload)
            | Self::QueueCancelled(payload)
            | Self::WriteDeferred(payload)
            | Self::Usage(payload) => payload,
        };
        payload
            .get("runId")
            .or_else(|| payload.get("id"))
            .or_else(|| payload.get("entryId"))
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
    }

    /// Return the owning operation ID when Pi defines one for this family.
    pub fn run_id(&self) -> Option<&str> {
        let payload = match self {
            Self::OperationStarted(payload)
            | Self::AbortRequested(payload)
            | Self::OperationFinished(payload)
            | Self::StepAttempt(payload)
            | Self::ToolStarted(payload)
            | Self::QueueEnqueued(payload)
            | Self::QueueCancelled(payload)
            | Self::WriteDeferred(payload)
            | Self::Usage(payload) => payload,
        };
        payload
            .get("runId")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
    }
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
    validate_operation_lane_record(snapshot, kind, data)?;
    validate_operation_finished_record(kind, data)?;
    validate_step_attempt_record(kind, data)?;
    validate_tool_started_record(snapshot, kind, data)?;
    validate_queue_lane_record(snapshot, kind, data)?;
    Ok(kind)
}

fn validate_operation_finished_record(
    kind: SessionLaneRecordKind,
    data: &serde_json::Value,
) -> Result<(), String> {
    if kind != SessionLaneRecordKind::OperationFinished {
        return Ok(());
    }
    let outcome = data
        .get("outcome")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "operation_finished is missing outcome".to_owned())?;
    if !matches!(outcome, "completed" | "aborted" | "failed" | "declined") {
        return Err(format!(
            "operation_finished has unknown outcome {outcome:?}"
        ));
    }
    if let Some(error) = data.get("error") {
        let code = error.get("code").and_then(serde_json::Value::as_str);
        let message = error.get("message").and_then(serde_json::Value::as_str);
        if code.is_none_or(str::is_empty) || message.is_none_or(str::is_empty) {
            return Err("operation_finished error requires code and message".into());
        }
    }
    Ok(())
}

fn validate_step_attempt_record(
    kind: SessionLaneRecordKind,
    data: &serde_json::Value,
) -> Result<(), String> {
    if kind != SessionLaneRecordKind::StepAttempt {
        return Ok(());
    }
    let step = data
        .get("step")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "step_attempt is missing step".to_owned())?;
    if !matches!(step, "assistant" | "branch_summary" | "compaction") {
        return Err(format!("step_attempt has unknown step {step:?}"));
    }
    let attempt = data
        .get("attempt")
        .and_then(serde_json::Value::as_u64)
        .filter(|value| *value > 0)
        .ok_or_else(|| "step_attempt has invalid attempt".to_owned())?;
    let _ = attempt;
    let has_result = data
        .get("resultEntryId")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.is_empty());
    if !has_result {
        return Err("step_attempt is missing resultEntryId".into());
    }
    match (step, data.get("compactionReason")) {
        ("compaction", Some(reason))
            if matches!(reason.as_str(), Some("manual" | "threshold" | "overflow")) => {}
        ("compaction", _) => return Err("compaction step has invalid compactionReason".into()),
        (_, Some(_)) => return Err("non-compaction step has compactionReason".into()),
        _ => {}
    }
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    reason = "the Pi tool-start contract keeps all linkage validation together"
)]
fn validate_tool_started_record(
    snapshot: &SessionSnapshot,
    kind: SessionLaneRecordKind,
    data: &serde_json::Value,
) -> Result<(), String> {
    if kind != SessionLaneRecordKind::ToolStarted {
        return Ok(());
    }
    // Legacy provider events are retained until the bridge can supply the
    // actor-owned assistant/result linkage. Complete Pi-shaped records take
    // the strict path below.
    let Some(assistant_id) = data
        .get("assistantEntryId")
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(());
    };
    let tool_index = data
        .get("toolIndex")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "tool_started is missing toolIndex".to_owned())?;
    let tool_call_id = data
        .get("toolCallId")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "tool_started is missing toolCallId".to_owned())?;
    let tool_name = data
        .get("toolName")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "tool_started is missing toolName".to_owned())?;
    let has_result = data
        .get("resultEntryId")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.is_empty());
    if !has_result {
        return Err("tool_started is missing resultEntryId".into());
    }
    if !matches!(
        data.get("replay").and_then(serde_json::Value::as_str),
        Some("never" | "safe")
    ) {
        return Err("tool_started has invalid replay policy".into());
    }
    let Some(entry) = snapshot
        .entries
        .iter()
        .find(|entry| entry.id == assistant_id)
    else {
        return Err(format!(
            "tool_started references unknown assistant {assistant_id:?}"
        ));
    };
    let AgentMessage::Assistant(assistant) = &entry.message else {
        return Err("tool_started assistantEntryId is not an assistant".into());
    };
    let Some(crate::types::AssistantContent::ToolCall(call)) =
        assistant.content.get(tool_index as usize)
    else {
        return Err(format!("tool_started has invalid toolIndex {tool_index}"));
    };
    if call.id != tool_call_id || call.name != tool_name {
        return Err("tool_started tool call identity does not match assistant entry".into());
    }
    let duplicate = snapshot.lane_records.iter().any(|record| {
        record.record_type == "tool_started"
            && record
                .data
                .get("assistantEntryId")
                .and_then(serde_json::Value::as_str)
                == Some(assistant_id)
            && record
                .data
                .get("toolIndex")
                .and_then(serde_json::Value::as_u64)
                == Some(tool_index)
    });
    if duplicate {
        return Err(format!(
            "tool invocation {assistant_id}:{tool_index} is duplicated"
        ));
    }
    Ok(())
}

fn validate_operation_lane_record(
    snapshot: &SessionSnapshot,
    kind: SessionLaneRecordKind,
    data: &serde_json::Value,
) -> Result<(), String> {
    if kind == SessionLaneRecordKind::OperationStarted {
        return Ok(());
    }
    let Some(run_id) = data
        .get("runId")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    if !snapshot.active_operations.contains_key(run_id) {
        return Err(format!("record references unknown operation {run_id:?}"));
    }
    let finished = snapshot.lane_records.iter().any(|record| {
        record.record_type == "operation_finished"
            && record.data.get("runId").and_then(serde_json::Value::as_str) == Some(run_id)
    });
    if finished {
        return Err(format!("record follows finished operation {run_id:?}"));
    }
    Ok(())
}

fn validate_queue_lane_record(
    snapshot: &SessionSnapshot,
    kind: SessionLaneRecordKind,
    data: &serde_json::Value,
) -> Result<(), String> {
    if kind == SessionLaneRecordKind::QueueEnqueued {
        let has_target_id = data
            .get("target")
            .and_then(|target| target.get("id"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.is_empty());
        return has_target_id
            .then_some(())
            .ok_or_else(|| "queue_enqueued is missing target.id".into());
    }
    if kind != SessionLaneRecordKind::QueueCancelled {
        return Ok(());
    }
    let entry_id = data
        .get("entryId")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "queue_cancelled is missing entryId".to_owned())?;
    let enqueue = snapshot.lane_records.iter().find(|record| {
        record.record_type == "queue_enqueued"
            && record
                .data
                .get("target")
                .and_then(|target| target.get("id"))
                .and_then(serde_json::Value::as_str)
                == Some(entry_id)
    });
    let Some(enqueue) = enqueue else {
        return Err(format!(
            "queue_cancelled references unknown entry {entry_id:?}"
        ));
    };
    let enqueue_run_id = enqueue
        .data
        .get("runId")
        .and_then(serde_json::Value::as_str);
    let cancel_run_id = data.get("runId").and_then(serde_json::Value::as_str);
    (enqueue_run_id == cancel_run_id)
        .then_some(())
        .ok_or_else(|| format!("queue_cancelled entry {entry_id:?} has mismatched runId"))
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
    let Ok(record) = SessionLaneRecord::decode(record_type, data) else {
        return;
    };
    if validate_session_lane_record(snapshot, record_type, data).is_err() {
        return;
    }
    let Some(record_id) = record.identity() else {
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
    if record.kind() == SessionLaneRecordKind::OperationStarted
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
    let operation_id = record.run_id().or_else(|| {
        matches!(
            record.kind(),
            SessionLaneRecordKind::OperationStarted
                | SessionLaneRecordKind::AbortRequested
                | SessionLaneRecordKind::OperationFinished
        )
        .then_some(record_id)
    });
    let Some(operation_id) = operation_id else {
        return;
    };
    match record {
        SessionLaneRecord::OperationStarted(_) => {
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
        SessionLaneRecord::AbortRequested(_) => {
            snapshot
                .active_operations
                .insert(operation_id.to_owned(), "aborted".into());
        }
        SessionLaneRecord::OperationFinished(_) => {
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

/// Select the operation that most recently entered the lane and is still
/// active. Pi correlates assistant steps with the current operation, not with
/// a lexical map key; lane order is the actor-owned source of that fact.
fn latest_active_operation(snapshot: &SessionSnapshot) -> Option<String> {
    snapshot
        .lane_records
        .iter()
        .rev()
        .find_map(|record| {
            (record.record_type == "operation_started"
                && snapshot.active_operations.contains_key(&record.id))
            .then(|| record.id.clone())
        })
        .or_else(|| snapshot.active_operations.keys().next_back().cloned())
}

impl SessionSnapshot {
    /// Return the selected branch from oldest to newest journal node.
    /// Message and configuration records share the same parent/id namespace.
    pub fn branch_entry_ids(&self) -> Vec<String> {
        self.branch_entry_ids_from_leaf(self.leaf_id.clone())
    }

    /// Return the selected branch for one Pi session lane.
    pub fn branch_entry_ids_for_lane(&self, lane: &str) -> Vec<String> {
        self.branch_entry_ids_from_leaf(self.lanes().get(lane).cloned().flatten())
    }

    /// Return message entries on one lane's selected branch, oldest first.
    pub fn entries_for_lane(&self, lane: &str) -> Vec<SessionEntry> {
        let ids = self
            .branch_entry_ids_for_lane(lane)
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        self.entries
            .iter()
            .filter(|entry| ids.contains(&entry.id))
            .cloned()
            .collect()
    }

    fn branch_entry_ids_from_leaf(&self, leaf_id: Option<String>) -> Vec<String> {
        let mut parents = BTreeMap::new();
        for entry in &self.entries {
            parents.insert(entry.id.clone(), entry.parent_id.clone());
        }
        for entry in &self.config_records {
            parents.insert(entry.id.clone(), entry.parent_id.clone());
        }
        let mut path = Vec::new();
        let mut current = leaf_id;
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
        self.fork_from_branch(target_id, self.branch_entry_ids())
    }

    /// Fork a validated message target from a named Pi session lane.
    pub fn fork_at_lane_message(&self, lane: &str, target_id: &str) -> Result<Self, String> {
        self.fork_from_branch(target_id, self.branch_entry_ids_for_lane(lane))
    }

    #[allow(clippy::too_many_lines)]
    fn fork_from_branch(&self, target_id: &str, branch: Vec<String>) -> Result<Self, String> {
        if !self.entries.iter().any(|entry| entry.id == target_id) {
            return Err(format!("invalid fork target {target_id:?}"));
        }
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
            if let Some(lane) = self.entry_lanes.get(&entry.id) {
                fork.entry_lanes.insert(copy.id.clone(), lane.clone());
            }
            fork.entries.push(copy);
        }
        for entry in &self.config_records {
            if !retained.contains(&entry.id)
                || matches!(
                    entry.record,
                    SessionConfigRecord::NameChanged { .. }
                        | SessionConfigRecord::LabelChanged { .. }
                )
            {
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
        // Pi forks publish a fresh main-lane pointer after the copied entry
        // prefix. Lane facts from the source are not copied verbatim; the
        // fork receives one authoritative pointer for its new tree.
        sequence += 1;
        fork.lane_facts.push(SessionLaneFact {
            seq: sequence,
            lane: "main".into(),
            leaf_id: Some(target_id.to_owned()),
        });
        if let Some(name) = self.name() {
            sequence += 1;
            fork.config_records.push(SessionConfigEntry {
                id: format!("fork-fact-{sequence}"),
                seq: sequence,
                parent_id: Some(target_id.to_owned()),
                timestamp: 0,
                record: SessionConfigRecord::NameChanged { name },
            });
        }
        for (label_target, label) in self
            .labels()
            .into_iter()
            .filter(|(id, _)| retained.contains(id))
        {
            sequence += 1;
            fork.config_records.push(SessionConfigEntry {
                id: format!("fork-fact-{sequence}"),
                seq: sequence,
                parent_id: Some(target_id.to_owned()),
                timestamp: 0,
                record: SessionConfigRecord::LabelChanged {
                    target_id: label_target,
                    label: Some(label),
                },
            });
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
            if value.get("kind").and_then(serde_json::Value::as_str) == Some("lane") {
                let seq = value
                    .get("seq")
                    .and_then(serde_json::Value::as_u64)
                    .ok_or_else(|| format!("session lane {} is missing seq", line_index + 2))?;
                if seq != snapshot.sequence + 1 {
                    return Err(format!(
                        "session lane {} has non-consecutive seq",
                        line_index + 2
                    ));
                }
                let lane = value
                    .get("lane")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| format!("session lane {} is missing lane", line_index + 2))?
                    .to_owned();
                let leaf_id = value
                    .get("leafId")
                    .cloned()
                    .filter(|value| !value.is_null())
                    .map(|value| {
                        value.as_str().map(str::to_owned).ok_or_else(|| {
                            format!("session lane {} has invalid leafId", line_index + 2)
                        })
                    })
                    .transpose()?;
                if let Some(leaf_id) = &leaf_id {
                    if !snapshot.entries.iter().any(|entry| entry.id == *leaf_id) {
                        return Err(format!(
                            "session lane {} has unknown leafId",
                            line_index + 2
                        ));
                    }
                }
                snapshot.sequence = seq;
                snapshot
                    .lane_facts
                    .push(SessionLaneFact { seq, lane, leaf_id });
                continue;
            }
            if value.get("kind").and_then(serde_json::Value::as_str) != Some("entry") {
                return Err(format!(
                    "unsupported session mutation at line {}",
                    line_index + 2
                ));
            }
            let entry_type = value
                .get("type")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| format!("session entry {} is missing type", line_index + 2))?;
            if session_lane_record_kind(entry_type).is_some() {
                let data = value.clone();
                reduce_operation_record(&mut snapshot, entry_type, &data);
                continue;
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
                    "label" => SessionConfigRecord::LabelChanged {
                        target_id: value
                            .get("targetId")
                            .and_then(serde_json::Value::as_str)
                            .ok_or_else(|| {
                                format!("session entry {} is missing targetId", line_index + 2)
                            })?
                            .to_owned(),
                        label: match value.get("label") {
                            None | Some(serde_json::Value::Null) => None,
                            Some(value) => Some(
                                value
                                    .as_str()
                                    .ok_or_else(|| {
                                        format!(
                                            "session entry {} has invalid label",
                                            line_index + 2
                                        )
                                    })?
                                    .to_owned(),
                            ),
                        },
                    },
                    "session_name" => SessionConfigRecord::NameChanged {
                        name: value
                            .get("name")
                            .and_then(serde_json::Value::as_str)
                            .ok_or_else(|| {
                                format!("session entry {} is missing name", line_index + 2)
                            })?
                            .to_owned(),
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
            let entry_lane = value
                .get("lane")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("main")
                .to_owned();
            snapshot.entry_lanes.insert(id.clone(), entry_lane);
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
                    "lane": self.entry_lane(&session_entry.id).unwrap_or("main"),
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
        entry_lines.extend(self.config_records.iter().filter(|session_entry| {
            !matches!(
                session_entry.record,
                SessionConfigRecord::OperationRecordCreated { .. }
            )
        }).map(|session_entry| {
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
                SessionConfigRecord::LabelChanged { target_id, label } => (
                    "label",
                    serde_json::json!({ "targetId": target_id, "label": label }),
                ),
                SessionConfigRecord::NameChanged { name } => (
                    "session_name",
                    serde_json::json!({ "name": name }),
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
        entry_lines.extend(self.lane_facts.iter().map(|fact| {
            serde_json::json!({
                "kind": "lane",
                "lane": fact.lane,
                "seq": fact.seq,
                "leafId": fact.leaf_id,
            })
            .to_string()
        }));
        entry_lines.extend(self.lane_records.iter().enumerate().map(|(index, record)| {
            let mut entry = record.data.clone();
            let object = entry
                .as_object_mut()
                .expect("session lane record payload must be an object");
            object.insert("kind".into(), serde_json::json!("entry"));
            object.insert(
                "lane".into(),
                serde_json::json!(record.lane.as_deref().unwrap_or("main")),
            );
            object.insert("type".into(), serde_json::json!(record.record_type));
            object.insert("id".into(), serde_json::json!(record.id));
            object.insert(
                "parentId".into(),
                index
                    .checked_sub(1)
                    .and_then(|previous| self.lane_records.get(previous))
                    .map(|previous| serde_json::json!(previous.id))
                    .unwrap_or(serde_json::Value::Null),
            );
            object.insert(
                "seq".into(),
                serde_json::json!(record.seq.unwrap_or(index as u64 + 1)),
            );
            object.insert(
                "timestamp".into(),
                serde_json::json!(record.timestamp.unwrap_or(0)),
            );
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
    Append(
        String,
        Box<AgentMessage>,
        bool,
        oneshot::Sender<Result<(), String>>,
    ),
    ToolStarted {
        tool_call_id: String,
        tool_name: String,
        args: serde_json::Value,
        reply: oneshot::Sender<()>,
    },
    Config(SessionConfigRecord, oneshot::Sender<Result<(), String>>),
    Lane {
        lane: String,
        leaf_id: Option<String>,
        create: bool,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Import(SessionSnapshot, oneshot::Sender<()>),
    Reset(oneshot::Sender<()>),
    Flush(oneshot::Sender<()>),
    PrepareCompaction {
        token_estimates: Vec<u64>,
        keep_recent_tokens: u64,
        reply: oneshot::Sender<Result<Option<CompactionPreparation>, String>>,
    },
}

#[derive(Debug, Clone)]
struct PendingToolStart {
    tool_call_id: String,
    tool_name: String,
    args: serde_json::Value,
}

fn materialize_tool_start(
    state: &SessionSnapshot,
    next_id: &mut u64,
    tool_result_ids: &mut HashMap<String, String>,
    pending: PendingToolStart,
) -> Option<serde_json::Value> {
    let assistant = state.entries.iter().rev().find_map(|entry| {
        if let AgentMessage::Assistant(message) = &entry.message {
            Some((entry.id.clone(), message))
        } else {
            None
        }
    })?;
    let (tool_index, _) = assistant
        .1
        .content
        .iter()
        .enumerate()
        .find(|(_, content)| {
            matches!(
                content,
                crate::types::AssistantContent::ToolCall(call)
                    if call.id == pending.tool_call_id
            )
        })?;
    let run_id = latest_active_operation(state)?;
    let result_entry_id = format!("entry-{next_id}");
    *next_id += 1;
    tool_result_ids.insert(pending.tool_call_id.clone(), result_entry_id.clone());
    Some(serde_json::json!({
        "runId": run_id,
        "assistantEntryId": assistant.0,
        "toolIndex": tool_index,
        "toolCallId": pending.tool_call_id,
        "toolName": pending.tool_name,
        "effectiveArgs": pending.args,
        "resultEntryId": result_entry_id,
        "replay": "never",
    }))
}

/// Rebuild actor-local tool-result reservations after a journal restore.
///
/// The reservation is operational state, not a second source of truth: the
/// durable `tool_started` lane record is authoritative and only starts that
/// do not yet have a message entry remain reserved.
fn rebuild_tool_result_reservations(
    state: &SessionSnapshot,
    tool_result_ids: &mut HashMap<String, String>,
) {
    let entry_ids = state
        .entries
        .iter()
        .map(|entry| entry.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    tool_result_ids.clear();
    for record in &state.lane_records {
        if record.record_type != "tool_started" {
            continue;
        }
        let Some(tool_call_id) = record
            .data
            .get("toolCallId")
            .and_then(|value| value.as_str())
        else {
            continue;
        };
        let Some(result_entry_id) = record
            .data
            .get("resultEntryId")
            .and_then(|value| value.as_str())
        else {
            continue;
        };
        if !entry_ids.contains(result_entry_id) {
            tool_result_ids.insert(tool_call_id.to_owned(), result_entry_id.to_owned());
        }
    }
}

enum StorageCommand {
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
        target_id: String,
        session_id: String,
        created_at: i64,
        cwd: String,
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
    #[allow(
        clippy::too_many_lines,
        reason = "storage mailbox keeps publication and recovery commands explicit"
    )]
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
                        StorageCommand::Load { path, reply } => {
                            let result = async {
                                let contents = tokio::fs::read_to_string(&path)
                                    .await
                                    .map_err(|error| format!("read session JSONL: {error}"))?;
                                let repaired = SessionSnapshot::repair_jsonl_torn_tail(&contents)?;
                                SessionSnapshot::from_jsonl(&repaired)
                            }
                            .await;
                            let _ = reply.send(result);
                        }
                        StorageCommand::Fork {
                            path,
                            snapshot,
                            target_id,
                            session_id,
                            created_at,
                            cwd,
                            reply,
                        } => {
                            let result = async {
                                let fork = snapshot.fork_at_message(&target_id)?;
                                let temporary = format!("{path}.tmp");
                                tokio::fs::write(
                                    &temporary,
                                    fork.to_jsonl(&session_id, created_at, &cwd),
                                )
                                .await
                                .map_err(|error| format!("stage forked session JSONL: {error}"))?;
                                if let Err(error) = tokio::fs::rename(&temporary, &path).await {
                                    let _ = tokio::fs::remove_file(&temporary).await;
                                    return Err(format!("publish forked session JSONL: {error}"));
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
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(StorageCommand::Fork {
                path: path.into(),
                snapshot: Box::new(snapshot.clone()),
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
            let mut tool_result_ids = HashMap::<String, String>::new();
            let mut pending_tool_starts = Vec::<PendingToolStart>::new();
            while let Some(command) = rx.recv().await {
                match command {
                    Command::Append(lane, message, terminate, reply) => {
                        let lane_leaf = state.lanes().get(&lane).cloned().flatten();
                        if lane != "main" && !state.lanes().contains_key(&lane) {
                            let _ = reply.send(Err(format!("session lane does not exist: {lane}")));
                            continue;
                        }
                        state.sequence += 1;
                        let id = match message.as_ref() {
                            AgentMessage::ToolResult(result) => tool_result_ids
                                .remove(&result.tool_call_id)
                                .unwrap_or_else(|| {
                                    let id = format!("entry-{next_id}");
                                    next_id += 1;
                                    id
                                }),
                            _ => {
                                let id = format!("entry-{next_id}");
                                next_id += 1;
                                id
                            }
                        };
                        let assistant = match message.as_ref() {
                            AgentMessage::Assistant(assistant) => Some(assistant.clone()),
                            _ => None,
                        };
                        // Pi journals the attempt before the result entry is
                        // committed. The actor has already reserved the
                        // entry identity, so this remains one ordered
                        // mailbox reduction rather than a post-hoc guess.
                        if assistant.is_some() {
                            if let Some(run_id) = latest_active_operation(&state) {
                                let attempt = state
                                    .lane_records
                                    .iter()
                                    .filter(|record| {
                                        record.record_type == "step_attempt"
                                            && record
                                                .data
                                                .get("runId")
                                                .and_then(serde_json::Value::as_str)
                                                == Some(run_id.as_str())
                                    })
                                    .count()
                                    + 1;
                                let data = serde_json::json!({
                                    "runId": run_id,
                                    "step": "assistant",
                                    "attempt": attempt,
                                    "resultEntryId": id,
                                });
                                reduce_operation_record(&mut state, "step_attempt", &data);
                            }
                        }
                        let entry = SessionEntry {
                            id: id.clone(),
                            seq: state.sequence,
                            parent_id: lane_leaf.clone().or_else(|| state.leaf_id.clone()),
                            timestamp: message.timestamp(),
                            message: *message,
                            terminate,
                        };
                        state.entry_lanes.insert(id.clone(), lane.clone());
                        if lane == "main" {
                            state.leaf_id = Some(id.clone());
                        }
                        state.entries.push(entry);
                        if let Some(assistant) = assistant {
                            let data = serde_json::json!({
                                "entryId": id,
                                "usage": serde_json::to_value(&assistant.usage)
                                    .unwrap_or(serde_json::Value::Null),
                            });
                            reduce_operation_record(&mut state, "usage", &data);
                            if assistant.stop_reason == Some(StopReason::Deferred) {
                                let data = serde_json::json!({
                                    "entryId": id.clone(),
                                    "target": {
                                        "id": id.clone(),
                                        "message": serde_json::to_value(&assistant)
                                            .unwrap_or(serde_json::Value::Null),
                                    },
                                    "deferred": assistant.deferred,
                                });
                                reduce_operation_record(&mut state, "write_deferred", &data);
                            }
                        }
                        let pending = std::mem::take(&mut pending_tool_starts);
                        for pending_tool in pending {
                            let retry = pending_tool.clone();
                            if let Some(data) = materialize_tool_start(
                                &state,
                                &mut next_id,
                                &mut tool_result_ids,
                                pending_tool,
                            ) {
                                reduce_operation_record(&mut state, "tool_started", &data);
                            } else {
                                pending_tool_starts.push(retry);
                            }
                        }
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(Ok(()));
                    }
                    Command::ToolStarted {
                        tool_call_id,
                        tool_name,
                        args,
                        reply,
                    } => {
                        let pending = PendingToolStart {
                            tool_call_id,
                            tool_name,
                            args,
                        };
                        let retry = pending.clone();
                        if let Some(data) = materialize_tool_start(
                            &state,
                            &mut next_id,
                            &mut tool_result_ids,
                            pending,
                        ) {
                            reduce_operation_record(&mut state, "tool_started", &data);
                        } else {
                            pending_tool_starts.push(retry);
                        }
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(());
                    }
                    Command::Config(record, reply) => {
                        if let SessionConfigRecord::LabelChanged { target_id, .. } = &record {
                            if !state.entries.iter().any(|entry| entry.id == *target_id) {
                                let _ = reply
                                    .send(Err(format!("label target does not exist: {target_id}")));
                                continue;
                            }
                        }
                        if let SessionConfigRecord::OperationRecordCreated { record_type, data } =
                            &record
                        {
                            if let Err(error) =
                                validate_session_lane_record(&state, record_type, data)
                            {
                                let _ = reply.send(Err(error));
                                continue;
                            }
                            reduce_operation_record(&mut state, record_type, data);
                            let _ = snapshot_tx.send(state.clone());
                            let _ = reply.send(Ok(()));
                            continue;
                        }
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
                        let _ = reply.send(Ok(()));
                    }
                    Command::Lane {
                        lane,
                        leaf_id,
                        create,
                        reply,
                    } => {
                        if lane.is_empty() {
                            let _ = reply.send(Err("session lane cannot be empty".into()));
                            continue;
                        }
                        if let Some(leaf_id) = &leaf_id {
                            if !state.entries.iter().any(|entry| entry.id == *leaf_id) {
                                let _ =
                                    reply.send(Err(format!("lane leaf does not exist: {leaf_id}")));
                                continue;
                            }
                        }
                        let exists = state.lanes().contains_key(&lane);
                        if create == exists {
                            let action = if create { "create" } else { "move" };
                            let reason = if create {
                                "already exists"
                            } else {
                                "does not exist"
                            };
                            let _ =
                                reply.send(Err(format!("cannot {action} lane {lane}: {reason}")));
                            continue;
                        }
                        state.sequence += 1;
                        state.lane_facts.push(SessionLaneFact {
                            seq: state.sequence,
                            lane,
                            leaf_id,
                        });
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(Ok(()));
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
                        rebuild_tool_result_reservations(&state, &mut tool_result_ids);
                        pending_tool_starts.clear();
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(());
                    }
                    Command::Reset(reply) => {
                        state = SessionSnapshot::default();
                        next_id = 1;
                        tool_result_ids.clear();
                        pending_tool_starts.clear();
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
                            Command::Append("main".into(), Box::new(message), terminate, reply)
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
                    AgentEvent::ToolExecutionStart {
                        tool_call_id,
                        tool_name,
                        args,
                    } => {
                        if !mailbox_ack!(tx, |reply| Command::ToolStarted {
                            tool_call_id,
                            tool_name,
                            args,
                            reply,
                        }) {
                            break;
                        }
                    }
                    AgentEvent::Reset if !mailbox_ack!(tx, Command::Reset) => break,
                    AgentEvent::Reset => {}
                    AgentEvent::SessionEntryAppended { lane, message } => {
                        let (reply_tx, reply_rx) = oneshot::channel();
                        if tx
                            .send(Command::Append(lane, Box::new(message), false, reply_tx))
                            .await
                            .is_err()
                            || reply_rx.await.is_err()
                        {
                            break;
                        }
                    }
                    _ => {
                        if let Some(record) = session_config_record!(&event) {
                            let (reply_tx, reply_rx) = oneshot::channel();
                            if tx.send(Command::Config(record, reply_tx)).await.is_err()
                                || reply_rx.await.is_err()
                            {
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
            Command::Append("main".into(), Box::new(message), false, reply)
        });
    }

    /// Append through a named Pi session lane. Invalid lanes are rejected at
    /// the actor boundary without publishing a partial entry.
    pub async fn append_to_lane(&self, lane: String, message: AgentMessage) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Command::Append(lane, Box::new(message), false, reply_tx))
            .await
            .map_err(|_| "session actor is closed".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session actor response was dropped".to_owned())?
    }

    /// Append a Pi custom journal entry through the session owner. Custom
    /// entries are opaque extension data: the actor journals and persists the
    /// payload but never interprets it as an agent message.
    pub async fn append_custom_entry(
        &self,
        custom_type: String,
        data: Option<serde_json::Value>,
    ) -> Result<(), String> {
        if custom_type.trim().is_empty() {
            return Err("custom session entry type cannot be empty".to_owned());
        }
        self.record_config(SessionConfigRecord::CustomSessionEntryCreated { custom_type, data })
            .await
    }

    /// Apply a session configuration fact through the owning mailbox.
    pub async fn record_config(&self, record: SessionConfigRecord) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Command::Config(record, reply_tx))
            .await
            .map_err(|_| "session actor is closed".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session actor response was dropped".to_owned())?
    }

    pub async fn record_lane(
        &self,
        lane: String,
        leaf_id: Option<String>,
        create: bool,
    ) -> Result<(), String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(Command::Lane {
                lane,
                leaf_id,
                create,
                reply: reply_tx,
            })
            .await
            .map_err(|_| "session actor is closed".to_owned())?;
        reply_rx
            .await
            .map_err(|_| "session actor response was dropped".to_owned())?
    }

    /// Apply session-owned configuration facts from a replay event sequence.
    /// The reducer remains the actor boundary; callers do not mutate the
    /// snapshot or manufacture message entries.
    #[allow(
        clippy::too_many_lines,
        clippy::cognitive_complexity,
        reason = "session event dispatch keeps each journal variant explicit"
    )]
    pub async fn apply_event(&self, event: &AgentEvent) -> Result<(), String> {
        if let AgentEvent::CustomSessionEntryCreated { custom_type, data } = event {
            self.append_custom_entry(custom_type.clone(), data.clone())
                .await
        } else if let Some(record) = session_config_record!(event) {
            self.record_config(record).await
        } else if let AgentEvent::SessionLaneChanged {
            lane,
            leaf_id,
            create,
        } = event
        {
            self.record_lane(lane.clone(), leaf_id.clone(), *create)
                .await
        } else if let AgentEvent::SessionEntryAppended { lane, message } = event {
            self.append_to_lane(lane.clone(), message.clone()).await
        } else if matches!(event, AgentEvent::Reset) {
            self.reset().await;
            Ok(())
        } else {
            Ok(())
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
    use crate::types::{
        AssistantContent, AssistantMessage, DeferredHandle, StopReason, ToolCall,
        ToolResultContent, ToolResultMessage, Usage, UserContent, UserMessage,
    };

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
    #[allow(clippy::too_many_lines)]
    async fn labels_are_event_reduced_validated_and_removed_by_fact() {
        let actor = SessionActor::new();
        actor.append(user("one")).await;
        actor
            .apply_event(&AgentEvent::SessionLabelChanged {
                target_id: "entry-1".into(),
                label: Some("important".into()),
            })
            .await
            .expect("label admission");
        assert_eq!(
            actor.snapshot().labels().get("entry-1"),
            Some(&"important".to_owned())
        );

        actor
            .apply_event(&AgentEvent::SessionLabelChanged {
                target_id: "entry-1".into(),
                label: None,
            })
            .await
            .expect("label removal");
        assert!(actor.snapshot().labels().is_empty());
        actor
            .apply_event(&AgentEvent::SessionNameChanged {
                name: "demo".into(),
            })
            .await
            .expect("name admission");
        assert_eq!(actor.snapshot().name().as_deref(), Some("demo"));
        let error = actor
            .apply_event(&AgentEvent::SessionLabelChanged {
                target_id: "missing".into(),
                label: Some("bad".into()),
            })
            .await
            .expect_err("missing target must be rejected");
        assert!(error.contains("label target does not exist"));
        actor
            .apply_event(&AgentEvent::SessionLabelChanged {
                target_id: "entry-1".into(),
                label: Some("important".into()),
            })
            .await
            .expect("label before fork");
        let fork = actor
            .snapshot()
            .fork_at_message("entry-1")
            .expect("fork facts");
        assert_eq!(fork.name().as_deref(), Some("demo"));
        assert_eq!(fork.labels().get("entry-1"), Some(&"important".to_owned()));
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn lane_events_are_validated_projected_and_jsonl_round_tripped() {
        let actor = SessionActor::new();
        actor.append(user("one")).await;
        actor
            .record_lane("feature".into(), Some("entry-1".into()), true)
            .await
            .expect("lane admission");
        actor
            .record_config(SessionConfigRecord::NameChanged {
                name: "after-lane".into(),
            })
            .await
            .expect("config after lane");
        assert_eq!(
            actor.snapshot().lanes().get("feature"),
            Some(&Some("entry-1".into()))
        );
        let jsonl = actor.snapshot().to_jsonl("s", 1, "/tmp");
        let serialized: Vec<u64> = jsonl
            .lines()
            .skip(1)
            .map(|line| {
                serde_json::from_str::<serde_json::Value>(line).unwrap()["seq"]
                    .as_u64()
                    .unwrap()
            })
            .collect();
        assert_eq!(serialized, vec![1, 2, 3]);
        let (_, _, imported) = SessionSnapshot::from_jsonl(&jsonl).expect("lane JSONL");
        assert_eq!(imported.lanes(), actor.snapshot().lanes());
        assert!(actor
            .record_lane("feature".into(), Some("entry-1".into()), true)
            .await
            .is_err());
        assert!(actor
            .record_lane("missing-lane".into(), None, false)
            .await
            .is_err());
        assert!(actor
            .record_lane("feature".into(), Some("missing".into()), false)
            .await
            .is_err());
        assert_eq!(actor.snapshot().lane_facts.len(), 1);
    }

    #[tokio::test]
    async fn append_to_lane_updates_only_that_lane_and_persists_identity() {
        let actor = SessionActor::new();
        actor.append(user("main")).await;
        actor
            .record_lane("feature".into(), Some("entry-1".into()), true)
            .await
            .expect("lane create");
        actor
            .append_to_lane("feature".into(), user("feature"))
            .await
            .expect("lane append");
        let snapshot = actor.snapshot();
        assert_eq!(snapshot.entries.len(), 2);
        assert_eq!(snapshot.entry_lane("entry-2"), Some("feature"));
        assert_eq!(snapshot.entries[1].parent_id.as_deref(), Some("entry-1"));
        assert_eq!(
            snapshot.branch_entry_ids_for_lane("feature"),
            vec!["entry-1", "entry-2"]
        );
        assert_eq!(
            snapshot
                .entries_for_lane("feature")
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["entry-1", "entry-2"]
        );
        assert_eq!(
            snapshot.lanes().get("feature"),
            Some(&Some("entry-2".into()))
        );
        let lane_fork = snapshot
            .fork_at_lane_message("feature", "entry-2")
            .expect("feature lane fork");
        assert_eq!(lane_fork.entries.len(), 2);
        assert_eq!(lane_fork.entry_lane("entry-2"), Some("feature"));
        let (_, _, imported) = SessionSnapshot::from_jsonl(&snapshot.to_jsonl("s", 1, "/tmp"))
            .expect("lane append JSONL");
        assert_eq!(imported.entry_lane("entry-2"), Some("feature"));
    }

    #[tokio::test]
    async fn append_custom_entry_is_actor_owned_and_round_trips() {
        let actor = SessionActor::new();
        actor.append(user("before")).await;
        actor
            .append_custom_entry(
                "replay.marker".into(),
                Some(serde_json::json!({"source": "yaml"})),
            )
            .await
            .expect("custom entry");

        let snapshot = actor.snapshot();
        let custom = snapshot.find_entries(&SessionEntryQuery {
            record_type: Some("custom".into()),
            custom_type: Some("replay.marker".into()),
            ..SessionEntryQuery::default()
        });
        assert_eq!(custom.len(), 1);
        let jsonl = snapshot.to_jsonl("session", 1, "/tmp");
        let (_, _, restored) = SessionSnapshot::from_jsonl(&jsonl).expect("custom JSONL");
        assert_eq!(
            restored.find_entries(&SessionEntryQuery {
                record_type: Some("custom".into()),
                custom_type: Some("replay.marker".into()),
                ..SessionEntryQuery::default()
            }),
            custom
        );
        assert!(actor.append_custom_entry(" ".into(), None).await.is_err());
    }

    #[tokio::test]
    async fn bus_message_end_and_reset_are_the_only_projection_inputs() {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        let _ = actor
            .record_config(SessionConfigRecord::OperationRecordCreated {
                record_type: "operation_started".into(),
                data: serde_json::json!({"id": "run-1"}),
            })
            .await;
        let _ = actor
            .record_config(SessionConfigRecord::OperationRecordCreated {
                record_type: "operation_started".into(),
                data: serde_json::json!({"id": "run-1"}),
            })
            .await;
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

    #[allow(
        clippy::too_many_lines,
        reason = "operation replay covers lifecycle and persistence"
    )]
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
        let before_rejected = actor.snapshot().lane_records.len();
        let rejected = actor
            .record_config(SessionConfigRecord::OperationRecordCreated {
                record_type: "operation_started".into(),
                data: serde_json::json!({"id": "op-1"}),
            })
            .await;
        assert!(rejected.is_err());
        assert_eq!(actor.snapshot().lane_records.len(), before_rejected);
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
        assert!(imported.config_records.iter().all(|entry| {
            !matches!(
                entry.record,
                SessionConfigRecord::OperationRecordCreated { .. }
            )
        }));
        assert_eq!(
            imported
                .lane_records
                .iter()
                .map(|record| record.record_type.as_str())
                .collect::<Vec<_>>(),
            vec!["operation_started", "abort_requested", "operation_finished"]
        );
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
    fn typed_lane_record_decode_preserves_family_and_payload() {
        let payload = serde_json::json!({"runId": "op-1", "seq": 3});
        let record =
            SessionLaneRecord::decode("tool_started", &payload).expect("known Pi lane family");
        assert_eq!(record.kind(), SessionLaneRecordKind::ToolStarted);
        assert_eq!(record.identity(), Some("op-1"));
        assert_eq!(record.run_id(), Some("op-1"));
        assert!(matches!(record, SessionLaneRecord::ToolStarted(value) if value == payload));
        assert!(SessionLaneRecord::decode("unknown", &payload).is_err());
    }

    #[test]
    fn typed_lane_identity_prefers_pi_record_shapes() {
        let entry = SessionLaneRecord::decode(
            "usage",
            &serde_json::json!({"entryId": "entry-1", "runId": "run-1"}),
        )
        .expect("usage record");
        assert_eq!(entry.identity(), Some("run-1"));
        assert_eq!(entry.run_id(), Some("run-1"));

        let queue = SessionLaneRecord::decode(
            "queue_cancelled",
            &serde_json::json!({"entryId": "entry-1"}),
        )
        .expect("queue cancellation");
        assert_eq!(queue.identity(), Some("entry-1"));
        assert_eq!(queue.run_id(), None);
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the query regression keeps Pi filter combinations explicit"
    )]
    fn lane_query_filters_pi_records_without_reordering_the_snapshot() {
        let mut snapshot = SessionSnapshot::default();
        for (record_type, id, seq, data) in [
            (
                "operation_started",
                "run-1",
                1,
                serde_json::json!({"id":"run-1","intent":{"kind":"run"}}),
            ),
            (
                "step_attempt",
                "run-1",
                2,
                serde_json::json!({"runId":"run-1","step":"assistant","attempt":1,"resultEntryId":"entry-1"}),
            ),
            (
                "operation_started",
                "run-2",
                3,
                serde_json::json!({"id":"run-2","intent":{"kind":"compaction"}}),
            ),
        ] {
            snapshot.lane_records.push(SessionLaneRecordSnapshot {
                record_type: record_type.into(),
                id: id.into(),
                lane: Some("main".into()),
                seq: Some(seq),
                timestamp: Some(seq as i64),
                data,
            });
        }

        let records = snapshot.find_lane_records(&SessionLaneQuery {
            run_id: Some("run-1".into()),
            after_seq: Some(1),
            newest_first: true,
            limit: Some(1),
            ..SessionLaneQuery::default()
        });
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record_type, "step_attempt");
        assert_eq!(records[0].seq, Some(2));

        let compactions = snapshot.find_lane_records(&SessionLaneQuery {
            record_type: Some("operation_started".into()),
            operation_kind: Some("compaction".into()),
            ..SessionLaneQuery::default()
        });
        assert_eq!(compactions[0].id, "run-2");
    }

    #[test]
    fn open_operation_query_returns_active_starts_newest_first() {
        let mut snapshot = SessionSnapshot::default();
        for (id, seq) in [("run-1", 1), ("run-2", 2)] {
            snapshot.lane_records.push(SessionLaneRecordSnapshot {
                record_type: "operation_started".into(),
                id: id.into(),
                lane: Some("main".into()),
                seq: Some(seq),
                timestamp: Some(seq as i64),
                data: serde_json::json!({"id": id}),
            });
            snapshot
                .active_operations
                .insert(id.into(), "started".into());
        }
        snapshot.active_operations.remove("run-1");
        let records = snapshot.find_open_operations("main", Some(2));
        assert_eq!(
            records
                .iter()
                .map(|record| record.id.as_str())
                .collect::<Vec<_>>(),
            ["run-2"]
        );
    }

    #[test]
    fn session_log_merges_entries_and_lane_records_by_sequence() {
        let mut snapshot = SessionSnapshot {
            entries: vec![SessionEntry {
                id: "entry-1".into(),
                seq: 1,
                parent_id: None,
                timestamp: 7,
                message: user("hello"),
                terminate: false,
            }],
            ..SessionSnapshot::default()
        };
        snapshot.lane_records.push(SessionLaneRecordSnapshot {
            record_type: "operation_started".into(),
            id: "run-1".into(),
            lane: Some("main".into()),
            seq: Some(2),
            timestamp: Some(7),
            data: serde_json::json!({"id": "run-1"}),
        });
        let log = snapshot.get_log(Some(0), Some(2));
        assert!(matches!(log[0], SessionLogItem::Entry { seq: 1, .. }));
        assert!(matches!(log[1], SessionLogItem::Record { seq: 2, .. }));
        assert!(snapshot
            .get_log(Some(1), None)
            .iter()
            .all(|item| matches!(item, SessionLogItem::Record { seq: 2, .. })));
    }

    #[test]
    fn entry_query_returns_message_and_config_lanes_in_pi_order() {
        let snapshot = SessionSnapshot {
            entries: vec![SessionEntry {
                id: "entry-1".into(),
                seq: 1,
                parent_id: None,
                timestamp: 7,
                message: user("hello"),
                terminate: false,
            }],
            config_records: vec![SessionConfigEntry {
                id: "entry-2".into(),
                seq: 2,
                parent_id: Some("entry-1".into()),
                timestamp: 7,
                record: SessionConfigRecord::CustomSessionEntryCreated {
                    custom_type: "note".into(),
                    data: Some(serde_json::json!({"ok": true})),
                },
            }],
            ..SessionSnapshot::default()
        };
        let entries = snapshot.find_entries(&SessionEntryQuery {
            after_seq: Some(0),
            ..SessionEntryQuery::default()
        });
        assert!(matches!(entries[0], SessionEntryRecord::Message(_)));
        assert!(matches!(entries[1], SessionEntryRecord::Config(_)));
        let custom = snapshot.find_entries(&SessionEntryQuery {
            record_type: Some("custom".into()),
            custom_type: Some("note".into()),
            newest_first: true,
            limit: Some(1),
            ..SessionEntryQuery::default()
        });
        assert_eq!(custom.len(), 1);
        assert!(matches!(custom[0], SessionEntryRecord::Config(_)));
    }

    #[test]
    fn session_stats_reduce_usage_records_like_pi() {
        let mut snapshot = SessionSnapshot {
            entries: vec![SessionEntry {
                id: "entry-1".into(),
                seq: 1,
                parent_id: None,
                timestamp: 7,
                message: user("hello"),
                terminate: false,
            }],
            ..SessionSnapshot::default()
        };
        snapshot.lane_records.push(SessionLaneRecordSnapshot {
            record_type: "usage".into(),
            id: "entry-1".into(),
            lane: Some("main".into()),
            seq: Some(2),
            timestamp: Some(7),
            data: serde_json::json!({
                "usage": {
                    "input": 10,
                    "output": 8,
                    "cacheRead": 3,
                    "cacheWrite": 2,
                    "totalTokens": 18,
                    "cost": {"total": 9.5}
                }
            }),
        });
        assert_eq!(
            snapshot.stats(),
            SessionStats {
                message_count: 1,
                cached_tokens: 3,
                uncached_tokens: 12,
                total_tokens: 18,
                cost_total: 9.5,
            }
        );
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

    #[test]
    fn operation_lane_records_require_an_active_operation() {
        let mut snapshot = SessionSnapshot::default();
        assert!(validate_session_lane_record(
            &snapshot,
            "step_attempt",
            &serde_json::json!({"runId": "missing-run"})
        )
        .is_err());
        snapshot
            .active_operations
            .insert("op-1".into(), "started".into());
        snapshot.lane_records.push(SessionLaneRecordSnapshot {
            record_type: "operation_finished".into(),
            id: "finish-1".into(),
            lane: None,
            seq: None,
            timestamp: None,
            data: serde_json::json!({"runId": "op-1"}),
        });
        assert!(validate_session_lane_record(
            &snapshot,
            "step_attempt",
            &serde_json::json!({"runId": "op-1"})
        )
        .is_err());
    }

    #[test]
    fn step_attempt_records_match_pi_shape() {
        let valid = serde_json::json!({
            "runId": "run-1",
            "step": "assistant",
            "attempt": 1,
            "resultEntryId": "entry-1"
        });
        assert!(validate_step_attempt_record(SessionLaneRecordKind::StepAttempt, &valid).is_ok());
        assert!(validate_step_attempt_record(
            SessionLaneRecordKind::StepAttempt,
            &serde_json::json!({
                "runId": "run-1", "step": "assistant", "attempt": 0,
                "resultEntryId": "entry-1"
            })
        )
        .is_err());
        assert!(validate_step_attempt_record(
            SessionLaneRecordKind::StepAttempt,
            &serde_json::json!({
                "runId": "run-1", "step": "compaction", "attempt": 1,
                "resultEntryId": "entry-1", "compactionReason": "threshold"
            })
        )
        .is_ok());
    }

    #[test]
    fn operation_finished_records_match_pi_outcomes_and_errors() {
        for outcome in ["completed", "aborted", "failed", "declined"] {
            assert!(validate_operation_finished_record(
                SessionLaneRecordKind::OperationFinished,
                &serde_json::json!({"outcome": outcome})
            )
            .is_ok());
        }
        assert!(validate_operation_finished_record(
            SessionLaneRecordKind::OperationFinished,
            &serde_json::json!({"outcome": "unknown"})
        )
        .is_err());
        assert!(validate_operation_finished_record(
            SessionLaneRecordKind::OperationFinished,
            &serde_json::json!({
                "outcome": "failed",
                "error": {"code": "provider", "message": "unavailable"}
            })
        )
        .is_ok());
        assert!(validate_operation_finished_record(
            SessionLaneRecordKind::OperationFinished,
            &serde_json::json!({"outcome": "failed", "error": {"code": "provider"}})
        )
        .is_err());
    }

    #[test]
    fn tool_started_records_validate_actor_linkage() {
        let mut snapshot = SessionSnapshot::default();
        snapshot.entries.push(SessionEntry {
            id: "assistant-1".into(),
            seq: 1,
            parent_id: None,
            timestamp: 0,
            message: AgentMessage::Assistant(AssistantMessage {
                content: vec![AssistantContent::ToolCall(ToolCall {
                    id: "call-1".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({"path": "Cargo.toml"}),
                    thought_signature: None,
                })],
                ..Default::default()
            }),
            terminate: false,
        });
        let valid = serde_json::json!({
            "assistantEntryId": "assistant-1", "toolIndex": 0,
            "toolCallId": "call-1", "toolName": "read",
            "resultEntryId": "result-1", "replay": "never"
        });
        assert!(validate_tool_started_record(
            &snapshot,
            SessionLaneRecordKind::ToolStarted,
            &valid
        )
        .is_ok());
        assert!(validate_tool_started_record(
            &snapshot,
            SessionLaneRecordKind::ToolStarted,
            &serde_json::json!({
                "assistantEntryId": "assistant-1", "toolIndex": 1,
                "toolCallId": "call-1", "toolName": "read",
                "resultEntryId": "result-1", "replay": "never"
            })
        )
        .is_err());
    }

    #[tokio::test]
    #[allow(
        clippy::too_many_lines,
        reason = "integration fixture spells out the complete Pi record"
    )]
    async fn actor_reduces_complete_tool_started_identity() {
        let actor = SessionActor::new();
        let _ = actor
            .record_config(SessionConfigRecord::OperationRecordCreated {
                record_type: "operation_started".into(),
                data: serde_json::json!({"id": "run-1"}),
            })
            .await;
        actor
            .append(AgentMessage::Assistant(AssistantMessage {
                content: vec![AssistantContent::ToolCall(ToolCall {
                    id: "call-1".into(),
                    name: "read".into(),
                    arguments: serde_json::json!({"path": "Cargo.toml"}),
                    thought_signature: None,
                })],
                ..Default::default()
            }))
            .await;
        let _ = actor
            .record_config(SessionConfigRecord::OperationRecordCreated {
                record_type: "tool_started".into(),
                data: serde_json::json!({
                    "runId": "run-1",
                    "assistantEntryId": "entry-1",
                    "toolIndex": 0,
                    "toolCallId": "call-1",
                    "toolName": "read",
                    "effectiveArgs": {"path": "Cargo.toml"},
                    "resultEntryId": "entry-2",
                    "replay": "never"
                }),
            })
            .await;
        let snapshot = actor.snapshot();
        let record = snapshot
            .lane_records
            .iter()
            .find(|record| record.record_type == "tool_started")
            .expect("complete tool-start record");
        assert_eq!(record.data["assistantEntryId"], "entry-1");
        assert_eq!(record.data["resultEntryId"], "entry-2");
        assert_eq!(record.data["replay"], "never");
    }

    #[test]
    fn queue_lane_records_require_a_linked_provisioned_target() {
        let mut snapshot = SessionSnapshot::default();
        assert!(validate_session_lane_record(
            &snapshot,
            "queue_enqueued",
            &serde_json::json!({"id": "queue-1", "queue": "steer"})
        )
        .is_err());
        snapshot
            .active_operations
            .insert("run-1".into(), "started".into());
        let enqueue = serde_json::json!({
            "id": "queue-1",
            "runId": "run-1",
            "queue": "steer",
            "target": {"id": "entry-1", "role": "user", "content": "hello"}
        });
        assert!(validate_session_lane_record(&snapshot, "queue_enqueued", &enqueue).is_ok());
        snapshot.lane_records.push(SessionLaneRecordSnapshot {
            record_type: "queue_enqueued".into(),
            id: "queue-1".into(),
            lane: None,
            seq: None,
            timestamp: None,
            data: enqueue,
        });
        assert!(validate_session_lane_record(
            &snapshot,
            "queue_cancelled",
            &serde_json::json!({"id": "cancel-1", "runId": "run-1", "entryId": "entry-1"})
        )
        .is_ok());
        assert!(validate_session_lane_record(
            &snapshot,
            "queue_cancelled",
            &serde_json::json!({"id": "cancel-2", "runId": "run-2", "entryId": "entry-1"})
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
    fn branch_entry_query_requires_start_and_respects_stop_and_limit() {
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
            ],
            leaf_id: Some("message-2".into()),
            ..SessionSnapshot::default()
        };
        let entries = snapshot
            .find_entries_on_branch(&SessionBranchEntryQuery {
                start: "message-2".into(),
                stop_at_id: Some("message-1".into()),
                newest_first: true,
                limit: Some(1),
                ..SessionBranchEntryQuery::default()
            })
            .expect("branch query");
        assert!(
            matches!(entries[0], SessionEntryRecord::Message(ref entry) if entry.id == "message-2")
        );
        assert!(snapshot
            .find_entries_on_branch(&SessionBranchEntryQuery {
                start: "missing".into(),
                ..SessionBranchEntryQuery::default()
            })
            .is_err());
    }

    #[test]
    fn singular_entry_queries_preserve_declared_order() {
        let snapshot = SessionSnapshot {
            entries: vec![SessionEntry {
                id: "message-1".into(),
                seq: 1,
                parent_id: None,
                timestamp: 0,
                message: user("one"),
                terminate: false,
            }],
            leaf_id: Some("message-1".into()),
            ..SessionSnapshot::default()
        };
        assert!(matches!(
            snapshot.find_entry(&SessionEntryQuery::default()),
            Some(SessionEntryRecord::Message(entry)) if entry.id == "message-1"
        ));
        assert!(matches!(
            snapshot
                .find_entry_on_branch(&SessionBranchEntryQuery {
                    start: "message-1".into(),
                    ..SessionBranchEntryQuery::default()
                })
                .expect("branch lookup"),
            Some(SessionEntryRecord::Message(entry)) if entry.id == "message-1"
        ));
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
        assert_eq!(fork.sequence, 3);
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
        assert_eq!(fork.lanes().get("main"), Some(&Some("message-2".into())));
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
    fn compaction_threshold_matches_pi_enabled_and_strict_boundary() {
        let settings = CompactionSettings {
            enabled: true,
            reserve_tokens: 100,
            keep_recent_tokens: 20,
        };
        assert!(!should_compact(900, 1_000, settings));
        assert!(!should_compact(100, 1_000, settings));
        assert!(should_compact(901, 1_000, settings));
        assert!(!should_compact(
            901,
            1_000,
            CompactionSettings {
                enabled: false,
                ..settings
            }
        ));
        assert!(should_compact(
            u64::MAX,
            10,
            CompactionSettings {
                reserve_tokens: u64::MAX,
                ..settings
            }
        ));
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the context usage vector keeps all Pi edge cases together"
    )]
    fn context_usage_prefers_latest_valid_assistant_usage_and_estimates_tail() {
        const EIGHT_TOKENS_OF_TEXT: &str = "12345678";
        const ONE_TOKEN_OF_TEXT: &str = "1234";
        let messages = vec![
            user(EIGHT_TOKENS_OF_TEXT),
            AgentMessage::Assistant(crate::types::AssistantMessage {
                usage: crate::types::Usage {
                    input: 10,
                    output: 5,
                    total_tokens: 0,
                    ..Default::default()
                },
                stop_reason: Some(StopReason::Stop),
                ..Default::default()
            }),
            user(ONE_TOKEN_OF_TEXT),
        ];
        assert_eq!(estimate_message_tokens(&messages[0]), 2);
        assert_eq!(estimate_message_tokens(&user("😀😀😀")), 2);
        assert_eq!(
            estimate_context_tokens(&messages),
            ContextUsageEstimate {
                tokens: 16,
                usage_tokens: 15,
                trailing_tokens: 1,
                last_usage_index: Some(1),
            }
        );
        let aborted = AgentMessage::Assistant(crate::types::AssistantMessage {
            usage: crate::types::Usage {
                total_tokens: 100,
                ..Default::default()
            },
            stop_reason: Some(StopReason::Aborted),
            ..Default::default()
        });
        assert_eq!(
            estimate_context_tokens(&[aborted]),
            ContextUsageEstimate {
                tokens: 0,
                usage_tokens: 0,
                trailing_tokens: 0,
                last_usage_index: None,
            }
        );
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
        let request = CompactionSummaryRequest::from_preparation(&preparation, &entries, None)
            .expect("provider request");
        assert_eq!(request.history, Vec::<AgentMessage>::new());
        assert_eq!(request.turn_prefix, vec![user("request")]);
        assert!(matches!(
            request.retained_tail.as_slice(),
            [AgentMessage::Assistant(_)]
        ));
        let invalid = CompactionPreparation {
            history_indices: vec![99],
            ..preparation
        };
        assert!(CompactionSummaryRequest::from_preparation(&invalid, &entries, None).is_err());
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the regression fixture spells out the parent-linked journal boundary"
    )]
    fn compaction_context_projection_drops_prefix_and_deferred_results() {
        let snapshot = SessionSnapshot {
            sequence: 4,
            leaf_id: Some("entry-4".into()),
            entries: vec![
                SessionEntry {
                    id: "entry-1".into(),
                    seq: 1,
                    parent_id: None,
                    timestamp: 0,
                    message: user("old"),
                    terminate: false,
                },
                SessionEntry {
                    id: "entry-2".into(),
                    seq: 2,
                    parent_id: Some("entry-1".into()),
                    timestamp: 0,
                    message: AgentMessage::Assistant(Default::default()),
                    terminate: false,
                },
                SessionEntry {
                    id: "entry-4".into(),
                    seq: 4,
                    parent_id: Some("entry-3".into()),
                    timestamp: 0,
                    message: AgentMessage::Assistant(AssistantMessage {
                        stop_reason: Some(StopReason::Deferred),
                        ..Default::default()
                    }),
                    terminate: false,
                },
            ],
            config_records: vec![SessionConfigEntry {
                id: "entry-3".into(),
                seq: 3,
                parent_id: Some("entry-2".into()),
                timestamp: 0,
                record: SessionConfigRecord::CompactionCreated {
                    summary: "summary".into(),
                    retained_tail: vec![user("retained")],
                    tokens_before: 80,
                    details: None,
                    usage: None,
                },
            }],
            ..Default::default()
        };
        let projection = snapshot
            .compaction_context_projection()
            .expect("latest compaction");
        assert_eq!(projection.summary, "summary");
        assert_eq!(projection.tokens_before, 80);
        assert_eq!(projection.retained_tail.len(), 1);
        assert!(projection.message_indices.is_empty());
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

    #[allow(
        clippy::too_many_lines,
        reason = "storage round-trip covers publish, load, and fork"
    )]
    #[tokio::test]
    async fn storage_actor_publishes_a_complete_jsonl_file_atomically() {
        let path = std::env::temp_dir().join(format!(
            "runie-session-{}-{}.jsonl",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let path_string = path.to_string_lossy().into_owned();
        let storage = SessionStorageActor::new();
        let snapshot = SessionSnapshot {
            sequence: 1,
            leaf_id: Some("entry-1".into()),
            entries: vec![SessionEntry {
                id: "entry-1".into(),
                seq: 1,
                parent_id: None,
                timestamp: 0,
                message: user("fork me"),
                terminate: false,
            }],
            ..SessionSnapshot::default()
        };
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
        let (session_id, cwd, loaded) = storage.load_snapshot(&path_string).await.expect("load");
        assert_eq!(session_id, "session-1");
        assert_eq!(cwd, "/tmp");
        assert_eq!(loaded.sequence, snapshot.sequence);
        tokio::fs::write(&path, format!("{contents}{{\"kind\":\"entry\""))
            .await
            .expect("tear final line");
        let (_, _, repaired) = storage
            .load_snapshot(&path_string)
            .await
            .expect("repair load");
        assert_eq!(repaired.sequence, snapshot.sequence);
        let fork_path = format!("{path_string}.fork");
        storage
            .fork_snapshot(&fork_path, &snapshot, "entry-1", "fork-1", 2, "/tmp")
            .await
            .expect("fork publish");
        let (_, _, forked) = storage.load_snapshot(&fork_path).await.expect("fork load");
        assert_eq!(forked.sequence, 2);
        assert_eq!(forked.leaf_id.as_deref(), Some("entry-1"));
        assert!(!tokio::fs::try_exists(format!("{path_string}.tmp"))
            .await
            .expect("temporary file check"));
        let _ = tokio::fs::remove_file(path).await;
        let _ = tokio::fs::remove_file(fork_path).await;
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
    #[allow(
        clippy::too_many_lines,
        reason = "integration fixture spells out tool lifecycle ordering"
    )]
    async fn bus_tool_termination_is_attached_to_the_owned_session_entry() {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        let _ = actor
            .record_config(SessionConfigRecord::OperationRecordCreated {
                record_type: "operation_started".into(),
                data: serde_json::json!({"id": "run-1"}),
            })
            .await;
        actor
            .append(AgentMessage::Assistant(AssistantMessage {
                content: vec![AssistantContent::ToolCall(ToolCall {
                    id: "call-1".into(),
                    name: "stop".into(),
                    arguments: serde_json::json!({"reason": "test"}),
                    thought_signature: None,
                })],
                ..Default::default()
            }))
            .await;
        let _ = actor
            .record_config(SessionConfigRecord::OperationRecordCreated {
                record_type: "tool_started".into(),
                data: serde_json::json!({
                    "runId": "run-1",
                    "assistantEntryId": "entry-1",
                    "toolIndex": 0,
                    "toolCallId": "call-1",
                    "toolName": "stop",
                    "effectiveArgs": {"reason": "test"},
                    "resultEntryId": "entry-2",
                    "replay": "never"
                }),
            })
            .await;
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
        assert_eq!(actor.snapshot().lane_records[3].record_type, "tool_started");
        assert_eq!(actor.snapshot().entries.len(), 2);
    }

    #[tokio::test]
    async fn assistant_message_end_emits_owned_usage_lane_record() {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        bus.publish(AgentEvent::OperationRecordCreated {
            record_type: "operation_started".into(),
            data: serde_json::json!({"id": "run-1", "lane": "main", "runId": "run-1"}),
        });
        bus.publish(AgentEvent::OperationRecordCreated {
            record_type: "operation_started".into(),
            data: serde_json::json!({"id": "run-2", "lane": "main", "runId": "run-2"}),
        });
        let assistant = AssistantMessage {
            usage: Usage::default(),
            ..Default::default()
        };
        bus.publish(AgentEvent::MessageEnd {
            message: AgentMessage::Assistant(assistant),
        });
        actor.flush().await;
        let snapshot = actor.snapshot();
        assert_eq!(
            snapshot
                .lane_records
                .iter()
                .map(|record| record.record_type.as_str())
                .collect::<Vec<_>>(),
            [
                "operation_started",
                "operation_started",
                "step_attempt",
                "usage"
            ]
        );
        assert_eq!(snapshot.lane_records[2].data["runId"], "run-2");
        assert_eq!(snapshot.lane_records[2].data["resultEntryId"], "entry-1");
        assert_eq!(snapshot.lane_records[3].id, "entry-1");
    }

    #[tokio::test]
    async fn deferred_assistant_commit_emits_write_deferred_lane_record() {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        let assistant = AssistantMessage {
            stop_reason: Some(StopReason::Deferred),
            deferred: Some(DeferredHandle {
                provider: "replay".into(),
                model_id: "model-1".into(),
                api: "replay-api".into(),
                id: "deferred-1".into(),
                expires_at: None,
                poll_after_ms: None,
                data: None,
            }),
            ..Default::default()
        };
        bus.publish(AgentEvent::MessageEnd {
            message: AgentMessage::Assistant(assistant),
        });
        actor.flush().await;
        assert_eq!(
            actor.snapshot().lane_records[1].record_type,
            "write_deferred"
        );
        assert_eq!(actor.snapshot().lane_records[1].id, "entry-1");
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
            entry_lanes: BTreeMap::from([(String::from("entry-1"), String::from("main"))]),
            config_records: Vec::new(),
            lane_facts: Vec::new(),
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
            entry_lanes: BTreeMap::new(),
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
            lane_facts: Vec::new(),
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

    #[tokio::test]
    #[allow(
        clippy::too_many_lines,
        reason = "the restore regression keeps the durable and live tool facts together"
    )]
    async fn actor_restore_rebuilds_unsettled_tool_result_reservation() {
        let source = SessionActor::new();
        let _ = source
            .record_config(SessionConfigRecord::OperationRecordCreated {
                record_type: "operation_started".into(),
                data: serde_json::json!({"id": "run-1"}),
            })
            .await;
        source
            .append(AgentMessage::Assistant(AssistantMessage {
                content: vec![AssistantContent::ToolCall(ToolCall {
                    id: "call-1".into(),
                    name: "echo".into(),
                    arguments: serde_json::json!({"value": "hello"}),
                    thought_signature: None,
                })],
                ..Default::default()
            }))
            .await;
        let _ = source
            .record_config(SessionConfigRecord::OperationRecordCreated {
                record_type: "tool_started".into(),
                data: serde_json::json!({
                    "runId": "run-1",
                    "assistantEntryId": "entry-1",
                    "toolIndex": 0,
                    "toolCallId": "call-1",
                    "toolName": "echo",
                    "effectiveArgs": {"value": "hello"},
                    "resultEntryId": "entry-3",
                    "replay": "never"
                }),
            })
            .await;
        let jsonl = source.snapshot().to_jsonl("session-1", 5, "/workspace");

        let restored = SessionActor::new();
        restored.restore_jsonl(&jsonl).await.expect("restore");
        restored
            .append(AgentMessage::ToolResult(ToolResultMessage {
                tool_call_id: "call-1".into(),
                tool_name: "echo".into(),
                content: vec![ToolResultContent::Text {
                    text: "hello".into(),
                }],
                details: serde_json::Value::Null,
                usage: None,
                added_tool_names: Vec::new(),
                is_error: false,
                timestamp: 7,
            }))
            .await;

        let snapshot = restored.snapshot();
        assert_eq!(
            snapshot.entries.last().map(|entry| entry.id.as_str()),
            Some("entry-3")
        );
        assert_eq!(snapshot.entries.len(), 2);
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
