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
    AdmitNavigation {
        operation_id: String,
        lane: String,
        target_id: String,
        summarize: bool,
        summary_entry_id: Option<String>,
        reply: oneshot::Sender<Result<(), String>>,
    },
    BeginCompaction {
        lane: String,
        reply: oneshot::Sender<Result<String, String>>,
    },
    Lane {
        lane: String,
        leaf_id: Option<String>,
        create: bool,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Fork {
        target_id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    SelectTree {
        target_id: String,
        reply: oneshot::Sender<Result<(), String>>,
    },
    Undo {
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
    PrepareAndBeginCompaction {
        token_estimates: Vec<u64>,
        keep_recent_tokens: u64,
        lane: String,
        reply: oneshot::Sender<Result<Option<PreparedCompaction>, String>>,
    },
    PublishCompaction {
        preparation: CompactionPreparation,
        summary: CompactionSummary,
        reply: oneshot::Sender<Result<(), String>>,
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
