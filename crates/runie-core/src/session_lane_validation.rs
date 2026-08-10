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
    apply_navigation_record(snapshot, record.kind(), data);
    let operation_id = record.run_id().map(str::to_owned).or_else(|| {
        matches!(
            record.kind(),
            SessionLaneRecordKind::OperationStarted
                | SessionLaneRecordKind::AbortRequested
                | SessionLaneRecordKind::OperationFinished
        )
        .then_some(record_id.to_owned())
    });
    let Some(operation_id) = operation_id else {
        return;
    };
    apply_operation_state(snapshot, record, &operation_id, data);
}

fn apply_navigation_record(
    snapshot: &mut SessionSnapshot,
    kind: SessionLaneRecordKind,
    data: &serde_json::Value,
) {
    if kind != SessionLaneRecordKind::OperationStarted
        || data
            .get("intent")
            .and_then(|intent| intent.get("kind"))
            .and_then(serde_json::Value::as_str)
            != Some("navigation")
    {
        return;
    }
    let Some(intent) = data.get("intent") else {
        return;
    };
    snapshot.navigation = Some(NavigationSnapshot {
        target_id: intent
            .get("targetId")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
        summarize: intent
            .get("summarize")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        summary_entry_id: intent
            .get("summaryEntryId")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned),
    });
}

fn apply_operation_state(
    snapshot: &mut SessionSnapshot,
    record: SessionLaneRecord,
    operation_id: &str,
    data: &serde_json::Value,
) {
    match record {
        SessionLaneRecord::OperationStarted(_) => {
            apply_operation_started(snapshot, operation_id, data)
        }
        SessionLaneRecord::AbortRequested(_) => {
            snapshot
                .active_operations
                .insert(operation_id.to_owned(), "aborted".into());
        }
        SessionLaneRecord::OperationFinished(_) => {
            apply_operation_finished(snapshot, operation_id, data)
        }
        _ => {}
    }
}

fn apply_operation_started(
    snapshot: &mut SessionSnapshot,
    operation_id: &str,
    data: &serde_json::Value,
) {
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

fn apply_operation_finished(
    snapshot: &mut SessionSnapshot,
    operation_id: &str,
    data: &serde_json::Value,
) {
    snapshot.active_operations.remove(operation_id);
    let Some(outcome) = data.get("outcome").and_then(serde_json::Value::as_str) else {
        return;
    };
    snapshot
        .operation_outcomes
        .insert(operation_id.to_owned(), outcome.to_owned());
    let Some(error) = data.get("error") else {
        return;
    };
    let (Some(code), Some(message)) = (
        error.get("code").and_then(serde_json::Value::as_str),
        error.get("message").and_then(serde_json::Value::as_str),
    ) else {
        return;
    };
    snapshot.operation_errors.insert(
        operation_id.to_owned(),
        OperationErrorSnapshot {
            code: code.to_owned(),
            message: message.to_owned(),
        },
    );
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
