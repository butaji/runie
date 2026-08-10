fn config_record_json(record: &SessionConfigRecord) -> (&str, serde_json::Value) {
    match record {
        SessionConfigRecord::ModelChanged { provider, model_id } => (
            "model_change",
            serde_json::json!({"provider": provider, "modelId": model_id}),
        ),
        SessionConfigRecord::ThinkingLevelChanged { level } => (
            "thinking_level_change",
            serde_json::json!({"thinkingLevel": level}),
        ),
        SessionConfigRecord::ActiveToolsChanged { tool_names } => (
            "active_tools_change",
            serde_json::json!({"activeToolNames": tool_names}),
        ),
        SessionConfigRecord::LabelChanged { target_id, label } => (
            "label",
            serde_json::json!({"targetId": target_id, "label": label}),
        ),
        SessionConfigRecord::NameChanged { name } => {
            ("session_name", serde_json::json!({"name": name}))
        }
        SessionConfigRecord::BranchSummaryCreated { .. } => config_branch_summary_json(record),
        SessionConfigRecord::CustomSessionEntryCreated { custom_type, data } => (
            "custom",
            serde_json::json!({"customType": custom_type, "data": data}),
        ),
        SessionConfigRecord::CompactionCreated { .. } => config_compaction_json(record),
        SessionConfigRecord::OperationRecordCreated { record_type, data } => {
            (record_type.as_str(), data.clone())
        }
        SessionConfigRecord::TypedOperation(operation) => {
            (operation.wire_name(), operation.data().clone())
        }
    }
}

fn config_branch_summary_json(record: &SessionConfigRecord) -> (&str, serde_json::Value) {
    let SessionConfigRecord::BranchSummaryCreated {
        from_id,
        summary,
        details,
    } = record
    else {
        unreachable!("branch serializer helper received another record")
    };
    (
        "branch_summary",
        serde_json::json!({
            "fromId": from_id, "summary": summary, "details": details,
        }),
    )
}

fn config_compaction_json(record: &SessionConfigRecord) -> (&str, serde_json::Value) {
    let SessionConfigRecord::CompactionCreated {
        summary,
        retained_tail,
        tokens_before,
        details,
        usage,
    } = record
    else {
        unreachable!("compaction serializer helper received another record")
    };
    (
        "compaction",
        serde_json::json!({
            "summary": summary, "retainedTail": retained_tail, "tokensBefore": tokens_before,
            "details": details, "usage": usage,
        }),
    )
}

type PreparedCompaction = (String, CompactionPreparation, Vec<SessionEntry>);

