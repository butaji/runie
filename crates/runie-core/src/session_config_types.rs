use super::super::*;
use crate::types::{AgentMessage, ThinkingLevel};

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
    /// Typed operation facts used inside the actor. This is lowered to the
    /// generic Pi `(record_type, data)` shape only at persistence/event edges.
    TypedOperation(SessionLaneRecord),
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
                match SessionLaneRecord::decode(record_type.as_ref(), &data) {
                    Ok(operation) => Some(SessionConfigRecord::TypedOperation(operation)),
                    Err(_) => Some(SessionConfigRecord::OperationRecordCreated {
                        record_type: record_type.clone(),
                        data: data.clone(),
                    }),
                }
            }
            AgentEvent::TypedOperationRecordCreated { kind, data } => {
                SessionLaneRecord::decode(kind.wire_name(), data)
                    .ok()
                    .map(SessionConfigRecord::TypedOperation)
            }
            _ => None,
        }
    }};
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionConfigEntry {
    pub id: String,
    /// Canonical Pi session-lane identity for configuration facts.
    pub lane: String,
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
    /// Canonical Pi session-lane identity for this message entry.
    pub lane: String,
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
    pub fn seq(&self) -> u64 {
        match self {
            Self::Message(entry) => entry.seq,
            Self::Config(entry) => entry.seq,
        }
    }

    pub fn record_type(&self) -> &'static str {
        match self {
            Self::Message(_) => "message",
            Self::Config(entry) => session_record_type(&entry.record),
        }
    }
}

macro_rules! session_record_types {
    ($(($pattern:pat => $wire_name:literal)),+ $(,)?) => {
        fn session_record_type(record: &SessionConfigRecord) -> &'static str {
            match record { $($pattern => $wire_name,)+ }
        }
    };
}

session_record_types! {
    (SessionConfigRecord::ModelChanged { .. } => "model_change"), (SessionConfigRecord::ThinkingLevelChanged { .. } => "thinking_level_change"),
    (SessionConfigRecord::ActiveToolsChanged { .. } => "active_tools_change"), (SessionConfigRecord::LabelChanged { .. } => "label"),
    (SessionConfigRecord::NameChanged { .. } => "session_name"), (SessionConfigRecord::BranchSummaryCreated { .. } => "branch_summary"),
    (SessionConfigRecord::CustomSessionEntryCreated { .. } => "custom"), (SessionConfigRecord::CompactionCreated { .. } => "compaction"),
    (SessionConfigRecord::OperationRecordCreated { .. } => "record"), (SessionConfigRecord::TypedOperation(_) => "record"),
}

/// Declarative equivalent of Pi's `EntryQuery`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionEntryQuery {
    /// Restrict message entries to one actor-owned session lane. Configuration
    /// records remain in the shared journal namespace.
    pub lane: Option<String>,
    pub record_type: Option<String>,
    pub custom_type: Option<String>,
    pub after_seq: Option<u64>,
    pub newest_first: bool,
    pub limit: Option<usize>,
}

/// Declarative Pi branch query. `start` is required; callers cannot silently
/// fall back to the current leaf when asking for a specific branch.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionBranchEntryQuery {
    pub start: String,
    /// Restrict message entries to the actor-owned session lane. Configuration
    /// facts remain part of the selected branch because Pi stores them in the
    /// shared journal namespace rather than duplicating them per lane.
    pub lane: Option<String>,
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

/// Lossless internal boundary for Pi lane records. Known families use the
/// validated typed union; extension records remain explicitly opaque instead
/// of leaking a bare `(record_type, data)` pair into actor consumers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionLaneRecordEnvelope {
    Known(SessionLaneRecord),
    Opaque {
        record_type: String,
        data: serde_json::Value,
    },
}

impl SessionLaneRecordEnvelope {
    pub fn record_type(&self) -> &str {
        match self {
            Self::Known(record) => record.wire_name(),
            Self::Opaque { record_type, .. } => record_type,
        }
    }

    pub fn data(&self) -> &serde_json::Value {
        match self {
            Self::Known(record) => record.data(),
            Self::Opaque { data, .. } => data,
        }
    }
}

impl SessionLaneRecordSnapshot {
    /// Decode the lossless wire payload at the actor-owned typed boundary.
    /// Persistence keeps `record_type` and `data` for Pi compatibility, while
    /// callers that need behavior use this one validated projection instead
    /// of matching JSON fields independently.
    pub fn typed_record(&self) -> Result<SessionLaneRecord, String> {
        SessionLaneRecord::decode(&self.record_type, &self.data)
    }

    /// Decode known families while preserving unknown Pi extension records.
    pub fn lossless_record(&self) -> SessionLaneRecordEnvelope {
        match self.typed_record() {
            Ok(record) => SessionLaneRecordEnvelope::Known(record),
            Err(_) => SessionLaneRecordEnvelope::Opaque {
                record_type: self.record_type.clone(),
                data: self.data.clone(),
            },
        }
    }

    pub fn kind(&self) -> Result<SessionLaneRecordKind, String> {
        self.typed_record().map(|record| record.kind())
    }
}

/// Declarative read boundary matching Pi's durable operation-lane query.
/// Filters are applied by the session owner to its immutable snapshot; the
/// query does not expose mutable journal state to callers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
    /// Entry identities used when this preparation was produced. The
    /// publication actor uses these to reject a stale plan after the journal
    /// has changed without relying on numeric indices alone.
    pub source_entry_ids: Vec<String>,
}

impl CompactionPreparation {
    fn validate_entries(&self, entries: &[SessionEntry]) -> Result<(), String> {
        if self.source_entry_ids.len() != entries.len()
            || self
                .source_entry_ids
                .iter()
                .zip(entries)
                .any(|(expected, actual)| expected != &actual.id)
        {
            return Err("compaction preparation is stale for the current journal".to_owned());
        }
        Ok(())
    }
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
    pub custom_instructions: Option<String>,
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
        preparation.validate_entries(entries)?;
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
            custom_instructions: None,
        })
    }

    pub fn with_custom_instructions(mut self, instructions: Option<String>) -> Self {
        self.custom_instructions = instructions;
        self
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

impl CompactionSummary {
    /// Convert a provider result and an actor-owned preparation into the
    /// journal event consumed by `SessionActor`.
    ///
    /// Retained messages are selected from the same immutable entry table
    /// used to build `CompactionSummaryRequest`; invalid indices are rejected
    /// before an event can cross the actor boundary.
    pub fn into_event(
        self,
        preparation: &CompactionPreparation,
        entries: &[SessionEntry],
    ) -> Result<crate::types::AgentEvent, String> {
        preparation.validate_entries(entries)?;
        let retained_tail = preparation
            .retained_indices
            .iter()
            .map(|index| {
                entries
                    .get(*index)
                    .map(|entry| entry.message.clone())
                    .ok_or_else(|| format!("compaction index {index} is out of bounds"))
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(crate::types::AgentEvent::CompactionCreated {
            summary: self.summary,
            retained_tail,
            tokens_before: preparation.tokens_before,
            details: self.details,
            usage: self.usage,
        })
    }
}
