fn validate_tool_started_record(
    snapshot: &SessionSnapshot,
    kind: SessionLaneRecordKind,
    data: &serde_json::Value,
) -> Result<(), String> {
    if kind != SessionLaneRecordKind::ToolStarted {
        return Ok(());
    }
    let Some((assistant_id, tool_index, tool_call_id, tool_name)) = tool_started_fields(data)?
    else {
        return Ok(());
    };
    validate_tool_started_linkage(snapshot, assistant_id, tool_index, tool_call_id, tool_name)
}

fn tool_started_fields(
    data: &serde_json::Value,
) -> Result<Option<(&str, u64, &str, &str)>, String> {
    let Some(assistant_id) = data
        .get("assistantEntryId")
        .and_then(serde_json::Value::as_str)
    else {
        return Ok(None);
    };
    let tool_index = data
        .get("toolIndex")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "tool_started is missing toolIndex".to_owned())?;
    let tool_call_id =
        required_tool_field(data, "toolCallId", "tool_started is missing toolCallId")?;
    let tool_name = required_tool_field(data, "toolName", "tool_started is missing toolName")?;
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
    Ok(Some((assistant_id, tool_index, tool_call_id, tool_name)))
}

fn required_tool_field<'a>(
    data: &'a serde_json::Value,
    name: &str,
    error: &str,
) -> Result<&'a str, String> {
    data.get(name)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| error.to_owned())
}

fn validate_tool_started_linkage(
    snapshot: &SessionSnapshot,
    assistant_id: &str,
    tool_index: u64,
    tool_call_id: &str,
    tool_name: &str,
) -> Result<(), String> {
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
    if tool_invocation_is_duplicate(snapshot, assistant_id, tool_index) {
        return Err(format!(
            "tool invocation {assistant_id}:{tool_index} is duplicated"
        ));
    }
    Ok(())
}

fn tool_invocation_is_duplicate(
    snapshot: &SessionSnapshot,
    assistant_id: &str,
    tool_index: u64,
) -> bool {
    snapshot.lane_records.iter().any(|record| {
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
    })
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
    if operation_is_finished(snapshot, run_id) {
        return Err(format!("record follows finished operation {run_id:?}"));
    }
    Ok(())
}

fn operation_is_finished(snapshot: &SessionSnapshot, run_id: &str) -> bool {
    snapshot.lane_records.iter().any(|record| {
        record.record_type == "operation_finished"
            && record.data.get("runId").and_then(serde_json::Value::as_str) == Some(run_id)
    })
}

fn validate_queue_lane_record(
    snapshot: &SessionSnapshot,
    kind: SessionLaneRecordKind,
    data: &serde_json::Value,
) -> Result<(), String> {
    if kind == SessionLaneRecordKind::QueueEnqueued {
        return validate_queue_enqueue(data);
    }
    if kind != SessionLaneRecordKind::QueueCancelled {
        return Ok(());
    }
    validate_queue_cancel(snapshot, data)
}

fn validate_queue_enqueue(data: &serde_json::Value) -> Result<(), String> {
    let has_target_id = data
        .get("target")
        .and_then(|target| target.get("id"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.is_empty());
    has_target_id
        .then_some(())
        .ok_or_else(|| "queue_enqueued is missing target.id".into())
}

fn validate_queue_cancel(
    snapshot: &SessionSnapshot,
    data: &serde_json::Value,
) -> Result<(), String> {
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
