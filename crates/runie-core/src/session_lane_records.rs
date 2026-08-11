/// Typed internal representation of a Pi operation-lane fact. The payload is
/// deliberately retained losslessly because Pi may add fields without a
/// Runie release; only the family is closed over here.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
        session_lane_record_kind(record_type)
            .map(|kind| Self::from_kind(kind, payload))
            .ok_or_else(|| format!("unknown session lane record type {record_type:?}"))
    }

    pub fn kind(&self) -> SessionLaneRecordKind {
        session_lane_record_kind_of(self)
    }

    pub fn wire_name(&self) -> &'static str {
        session_lane_record_wire_name(self.kind())
    }

    pub fn data(&self) -> &serde_json::Value {
        match self {
            Self::OperationStarted(data)
            | Self::AbortRequested(data)
            | Self::OperationFinished(data)
            | Self::StepAttempt(data)
            | Self::ToolStarted(data)
            | Self::QueueEnqueued(data)
            | Self::QueueCancelled(data)
            | Self::WriteDeferred(data)
            | Self::Usage(data) => data,
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

fn operation_record_parts(record: &SessionConfigRecord) -> Option<(&str, &serde_json::Value)> {
    match record {
        SessionConfigRecord::OperationRecordCreated { record_type, data } => {
            Some((record_type.as_str(), data))
        }
        SessionConfigRecord::TypedOperation(operation) => {
            Some((operation.wire_name(), operation.data()))
        }
        _ => None,
    }
}

pub fn session_lane_record_kind(record_type: &str) -> Option<SessionLaneRecordKind> {
    session_lane_record_kind_from_wire(record_type)
}

macro_rules! session_lane_records {
    ($(($variant:ident, $wire_name:literal)),+ $(,)?) => {
        impl SessionLaneRecord {
            fn from_kind(kind: SessionLaneRecordKind, data: serde_json::Value) -> Self {
                match kind { $(SessionLaneRecordKind::$variant => Self::$variant(data),)+ }
            }
        }

        fn session_lane_record_kind_of(record: &SessionLaneRecord) -> SessionLaneRecordKind {
            match record { $(SessionLaneRecord::$variant(_) => SessionLaneRecordKind::$variant,)+ }
        }

        fn session_lane_record_wire_name(kind: SessionLaneRecordKind) -> &'static str {
            match kind { $(SessionLaneRecordKind::$variant => $wire_name,)+ }
        }

        fn session_lane_record_kind_from_wire(value: &str) -> Option<SessionLaneRecordKind> {
            Some(match value { $($wire_name => SessionLaneRecordKind::$variant,)+ _ => return None })
        }
    };
}

session_lane_records! {
    (OperationStarted, "operation_started"),
    (AbortRequested, "abort_requested"),
    (OperationFinished, "operation_finished"),
    (StepAttempt, "step_attempt"),
    (ToolStarted, "tool_started"),
    (QueueEnqueued, "queue_enqueued"),
    (QueueCancelled, "queue_cancelled"),
    (WriteDeferred, "write_deferred"),
    (Usage, "usage"),
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
    validate_operation_started_record(snapshot, kind, data)?;
    validate_operation_lane_record(snapshot, kind, data)?;
    validate_operation_finished_record(kind, data)?;
    validate_step_attempt_record(kind, data)?;
    validate_tool_started_record(snapshot, kind, data)?;
    validate_queue_lane_record(snapshot, kind, data)?;
    Ok(kind)
}

fn validate_operation_started_record(
    snapshot: &SessionSnapshot,
    kind: SessionLaneRecordKind,
    data: &serde_json::Value,
) -> Result<(), String> {
    if kind != SessionLaneRecordKind::OperationStarted {
        return Ok(());
    }
    let Some(operation_kind) = data
        .pointer("/intent/kind")
        .and_then(serde_json::Value::as_str)
    else {
        // Application events historically carry only an operation id. The
        // durable Pi JSONL path supplies storage metadata and is validated
        // strictly; preserve the compatibility event boundary here.
        return Ok(());
    };
    if !matches!(operation_kind, "run" | "compaction" | "navigation") {
        return Err(format!(
            "operation_started has unknown operation kind {operation_kind:?}"
        ));
    }
    let Some(lane) = data
        .get("lane")
        .and_then(serde_json::Value::as_str)
        .filter(|lane| !lane.is_empty())
    else {
        return Ok(());
    };
    if operation_lane_is_open(snapshot, lane) {
        return Err(format!(
            "operation lane {lane:?} already has an open operation"
        ));
    }
    Ok(())
}

fn operation_lane_is_open(snapshot: &SessionSnapshot, lane: &str) -> bool {
    snapshot.active_operations.keys().any(|operation_id| {
        snapshot.lane_records.iter().any(|record| {
            record.record_type == "operation_started"
                && record.id == *operation_id
                && record.data.get("lane").and_then(serde_json::Value::as_str) == Some(lane)
        })
    })
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
