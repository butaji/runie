use super::super::*;
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

pub fn parse_jsonl_header(input: &str) -> Result<(String, String, Vec<&str>), String> {
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
    let session_id = required_header_string(&header, "id", "session header is missing id")?;
    let cwd = required_header_string(&header, "cwd", "session header is missing cwd")?;
    Ok((session_id, cwd, lines.collect()))
}

fn required_header_string(
    header: &serde_json::Value,
    field: &str,
    error: &str,
) -> Result<String, String> {
    header
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| error.to_owned())
}

fn parse_lane_fact(
    snapshot: &mut SessionSnapshot,
    value: &serde_json::Value,
    line_number: usize,
) -> Result<bool, String> {
    if value.get("kind").and_then(serde_json::Value::as_str) != Some("lane") {
        return Ok(false);
    }
    let seq = value
        .get("seq")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("session lane {line_number} is missing seq"))?;
    if seq != snapshot.sequence + 1 {
        return Err(format!(
            "session lane {line_number} has non-consecutive seq"
        ));
    }
    let lane = value
        .get("lane")
        .and_then(serde_json::Value::as_str)
        .filter(|lane| !lane.is_empty())
        .ok_or_else(|| format!("session lane {line_number} is missing lane"))?
        .to_owned();
    let leaf_id = parse_lane_leaf(value, line_number)?;
    validate_lane_leaf(snapshot, leaf_id.as_deref(), line_number)?;
    snapshot.sequence = seq;
    snapshot
        .lane_facts
        .push(SessionLaneFact { seq, lane, leaf_id });
    Ok(true)
}

fn parse_lane_leaf(
    value: &serde_json::Value,
    line_number: usize,
) -> Result<Option<String>, String> {
    value
        .get("leafId")
        .cloned()
        .filter(|value| !value.is_null())
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("session lane {line_number} has invalid leafId"))
        })
        .transpose()
}

fn validate_lane_leaf(
    snapshot: &SessionSnapshot,
    leaf_id: Option<&str>,
    line_number: usize,
) -> Result<(), String> {
    if leaf_id.is_some_and(|id| !snapshot.entries.iter().any(|entry| entry.id == id)) {
        return Err(format!("session lane {line_number} has unknown leafId"));
    }
    Ok(())
}

fn parse_operation_lane_record(
    snapshot: &mut SessionSnapshot,
    entry_type: &str,
    value: &serde_json::Value,
    line_number: usize,
) -> Result<bool, String> {
    if session_lane_record_kind(entry_type).is_none() {
        return Ok(false);
    }
    let data = value.clone();
    let seq = data
        .get("seq")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("session lane record {line_number} is missing seq"))?;
    if seq == 0 || (snapshot.sequence == 0 && seq != 1) || seq < snapshot.sequence {
        return Err(format!(
            "session lane record {line_number} has invalid sequence order"
        ));
    }
    data.get("lane")
        .and_then(serde_json::Value::as_str)
        .filter(|lane| !lane.is_empty())
        .ok_or_else(|| format!("session lane record {line_number} is missing lane"))?;
    validate_session_lane_record(snapshot, entry_type, &data)
        .map_err(|error| format!("invalid session lane record {line_number}: {error}"))?;
    reduce_operation_record(snapshot, entry_type, &data);
    snapshot.sequence = seq;
    Ok(true)
}

fn parse_entry_metadata(
    snapshot: &SessionSnapshot,
    value: &serde_json::Value,
    line_number: usize,
) -> Result<(u64, Option<String>, String, i64), String> {
    let seq = value
        .get("seq")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("session entry {line_number} is missing seq"))?;
    if seq != snapshot.sequence + 1 {
        return Err(format!(
            "session entry {line_number} has non-consecutive seq"
        ));
    }
    let parent_id = parse_parent_id(value, line_number)?;
    if parent_id != snapshot.leaf_id {
        return Err(format!(
            "session entry {line_number} has broken parent link"
        ));
    }
    let id = value
        .get("id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("session entry {line_number} is missing id"))?
        .to_owned();
    let timestamp = value
        .get("timestamp")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| format!("session entry {line_number} has invalid timestamp"))?;
    Ok((seq, parent_id, id, timestamp))
}

fn append_message_entry(
    snapshot: &mut SessionSnapshot,
    value: &serde_json::Value,
    seq: u64,
    parent_id: Option<String>,
    id: String,
    timestamp: i64,
    line_number: usize,
) -> Result<(), String> {
    let message = serde_json::from_value(
        value
            .get("message")
            .cloned()
            .ok_or_else(|| format!("session entry {line_number} is missing message"))?,
    )
    .map_err(|error| format!("session entry {line_number} has invalid message: {error}"))?;
    let terminate = value
        .get("terminate")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    snapshot.sequence = seq;
    snapshot.leaf_id = Some(id.clone());
    let lane = value
        .get("lane")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("main")
        .to_owned();
    snapshot.entry_lanes.insert(id.clone(), lane.clone());
    snapshot.entries.push(SessionEntry {
        id,
        lane,
        seq,
        parent_id,
        timestamp,
        message,
        terminate,
    });
    Ok(())
}

fn parse_model_change(
    value: &serde_json::Value,
    line_number: usize,
) -> Result<SessionConfigRecord, String> {
    let field = |name, error| required_json_string(value, name, line_number, error);
    Ok(SessionConfigRecord::ModelChanged {
        provider: field("provider", "provider")?,
        model_id: field("modelId", "modelId")?,
    })
}

fn parse_thinking_level_change(
    value: &serde_json::Value,
    line_number: usize,
) -> Result<SessionConfigRecord, String> {
    let raw = value
        .get("thinkingLevel")
        .cloned()
        .ok_or_else(|| format!("session entry {line_number} is missing thinkingLevel"))?;
    serde_json::from_value(raw)
        .map(|level| SessionConfigRecord::ThinkingLevelChanged { level })
        .map_err(|error| format!("invalid thinkingLevel: {error}"))
}

fn parse_active_tools_change(
    value: &serde_json::Value,
    line_number: usize,
) -> Result<SessionConfigRecord, String> {
    let raw = value
        .get("activeToolNames")
        .cloned()
        .ok_or_else(|| format!("session entry {line_number} is missing activeToolNames"))?;
    serde_json::from_value(raw)
        .map(|tool_names| SessionConfigRecord::ActiveToolsChanged { tool_names })
        .map_err(|error| format!("invalid activeToolNames: {error}"))
}

fn required_json_string(
    value: &serde_json::Value,
    field: &str,
    line_number: usize,
    label: &str,
) -> Result<String, String> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("session entry {line_number} is missing {label}"))
}

fn parse_label_change(
    value: &serde_json::Value,
    line_number: usize,
) -> Result<SessionConfigRecord, String> {
    let target_id = required_json_string(value, "targetId", line_number, "targetId")?;
    let label = match value.get("label") {
        None | Some(serde_json::Value::Null) => None,
        Some(value) => Some(
            value
                .as_str()
                .ok_or_else(|| format!("session entry {line_number} has invalid label"))?
                .to_owned(),
        ),
    };
    Ok(SessionConfigRecord::LabelChanged { target_id, label })
}

fn parse_name_change(
    value: &serde_json::Value,
    line_number: usize,
) -> Result<SessionConfigRecord, String> {
    Ok(SessionConfigRecord::NameChanged {
        name: required_json_string(value, "name", line_number, "name")?,
    })
}

fn parse_branch_summary(
    value: &serde_json::Value,
    line_number: usize,
) -> Result<SessionConfigRecord, String> {
    Ok(SessionConfigRecord::BranchSummaryCreated {
        from_id: required_json_string(value, "fromId", line_number, "fromId")?,
        summary: required_json_string(value, "summary", line_number, "summary")?,
        details: value
            .get("details")
            .cloned()
            .filter(|value| !value.is_null()),
    })
}

fn parse_custom_entry(
    value: &serde_json::Value,
    line_number: usize,
) -> Result<SessionConfigRecord, String> {
    Ok(SessionConfigRecord::CustomSessionEntryCreated {
        custom_type: required_json_string(value, "customType", line_number, "customType")?,
        data: value.get("data").cloned().filter(|value| !value.is_null()),
    })
}

fn parse_compaction(
    value: &serde_json::Value,
    line_number: usize,
) -> Result<SessionConfigRecord, String> {
    let summary = required_json_string(value, "summary", line_number, "summary")?;
    let retained_tail = serde_json::from_value(
        value
            .get("retainedTail")
            .cloned()
            .ok_or_else(|| format!("session entry {line_number} is missing retainedTail"))?,
    )
    .map_err(|error| format!("invalid retainedTail: {error}"))?;
    let tokens_before = value
        .get("tokensBefore")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| format!("session entry {line_number} is missing tokensBefore"))?;
    let details = value
        .get("details")
        .cloned()
        .filter(|value| !value.is_null());
    let usage = value
        .get("usage")
        .cloned()
        .filter(|value| !value.is_null())
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| format!("invalid usage: {error}"))?;
    Ok(SessionConfigRecord::CompactionCreated {
        summary,
        retained_tail,
        tokens_before,
        details,
        usage,
    })
}

fn parse_generic_operation(entry_type: &str, value: &serde_json::Value) -> SessionConfigRecord {
    SessionConfigRecord::OperationRecordCreated {
        record_type: entry_type.to_owned(),
        data: value.clone(),
    }
}

struct ConfigEntryInput {
    entry_type: String,
    value: serde_json::Value,
    seq: u64,
    parent_id: Option<String>,
    id: String,
    timestamp: i64,
    line_number: usize,
}

fn append_config_entry(
    snapshot: &mut SessionSnapshot,
    input: ConfigEntryInput,
) -> Result<(), String> {
    let record = parse_config_record(&input.entry_type, &input.value, input.line_number)?;
    snapshot.sequence = input.seq;
    snapshot.leaf_id = Some(input.id.clone());
    reduce_operation_record(snapshot, &input.entry_type, &input.value);
    let lane = input
        .value
        .get("lane")
        .and_then(serde_json::Value::as_str)
        .filter(|lane| !lane.is_empty())
        .unwrap_or("main")
        .to_owned();
    snapshot.config_records.push(SessionConfigEntry {
        id: input.id,
        lane,
        seq: input.seq,
        parent_id: input.parent_id,
        timestamp: input.timestamp,
        record,
    });
    Ok(())
}

fn parse_config_record(
    entry_type: &str,
    value: &serde_json::Value,
    line_number: usize,
) -> Result<SessionConfigRecord, String> {
    match entry_type {
        "model_change" => parse_model_change(value, line_number),
        "thinking_level_change" => parse_thinking_level_change(value, line_number),
        "active_tools_change" => parse_active_tools_change(value, line_number),
        "label" => parse_label_change(value, line_number),
        "session_name" => parse_name_change(value, line_number),
        "branch_summary" => parse_branch_summary(value, line_number),
        "custom" => parse_custom_entry(value, line_number),
        "compaction" => parse_compaction(value, line_number),
        _ => Ok(parse_generic_operation(entry_type, value)),
    }
}

pub fn parse_jsonl_line(
    snapshot: &mut SessionSnapshot,
    line: &str,
    line_number: usize,
) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_str(line)
        .map_err(|error| format!("invalid session entry {line_number}: {error}"))?;
    if parse_lane_fact(snapshot, &value, line_number)? {
        return Ok(());
    }
    if value.get("kind").and_then(serde_json::Value::as_str) != Some("entry") {
        return Err(format!(
            "unsupported session mutation at line {line_number}"
        ));
    }
    let entry_type = value
        .get("type")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("session entry {line_number} is missing type"))?;
    if parse_operation_lane_record(snapshot, entry_type, &value, line_number)? {
        return Ok(());
    }
    let (seq, parent_id, id, timestamp) = parse_entry_metadata(snapshot, &value, line_number)?;
    if entry_type != "message" {
        append_config_entry(
            snapshot,
            ConfigEntryInput {
                entry_type: entry_type.to_owned(),
                value: value.clone(),
                seq,
                parent_id,
                id,
                timestamp,
                line_number,
            },
        )
    } else {
        append_message_entry(snapshot, &value, seq, parent_id, id, timestamp, line_number)
    }
}

fn parse_parent_id(
    value: &serde_json::Value,
    line_number: usize,
) -> Result<Option<String>, String> {
    let parent = value
        .get("parentId")
        .ok_or_else(|| format!("session entry {line_number} is missing parentId"))?;
    if parent.is_null() {
        return Ok(None);
    }
    parent
        .as_str()
        .map(|value| Some(value.to_owned()))
        .ok_or_else(|| format!("session entry {line_number} has invalid parentId"))
}
