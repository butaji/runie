macro_rules! session_worker {
    ($snapshot_tx:expr, $rx:expr) => {{
        let snapshot_tx = $snapshot_tx;
        let mut rx = $rx;
        async move {

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
                            lane: lane.clone(),
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
                        if let Some((record_type, data)) = operation_record_parts(&record) {
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
                            lane: "main".into(),
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
                    Command::AdmitNavigation {
                        operation_id,
                        lane,
                        target_id,
                        summarize,
                        summary_entry_id,
                        reply,
                    } => {
                        let exists = |id: &str| {
                            state.entries.iter().any(|entry| entry.id == id)
                                || state.config_records.iter().any(|entry| entry.id == id)
                        };
                        if !exists(&target_id) {
                            let _ = reply.send(Err(format!(
                                "navigation target does not exist: {target_id}"
                            )));
                            continue;
                        }
                        if let Some(summary_id) = &summary_entry_id {
                            if !exists(summary_id) {
                                let _ = reply.send(Err(format!(
                                    "navigation summary entry does not exist: {summary_id}"
                                )));
                                continue;
                            }
                        }
                        let record = SessionConfigRecord::TypedOperation(
                            SessionLaneRecord::OperationStarted(serde_json::json!({
                                "id": operation_id,
                                "lane": lane,
                                "intent": {
                                    "kind": "navigation",
                                    "targetId": target_id,
                                    "summarize": summarize,
                                    "summaryEntryId": summary_entry_id,
                                },
                            })),
                        );
                        let Some((record_type, data)) = operation_record_parts(&record) else {
                            let _ = reply
                                .send(Err("navigation operation could not be encoded".to_owned()));
                            continue;
                        };
                        if let Err(error) = validate_session_lane_record(&state, record_type, data)
                        {
                            let _ = reply.send(Err(error));
                            continue;
                        }
                        reduce_operation_record(&mut state, record_type, data);
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(Ok(()));
                    }
                    Command::BeginCompaction { lane, reply } => {
                        let next = state
                            .lane_records
                            .iter()
                            .filter_map(|record| record.id.strip_prefix("compaction-"))
                            .filter_map(|value| value.parse::<u64>().ok())
                            .max()
                            .unwrap_or_default()
                            .saturating_add(1);
                        let operation_id = format!("compaction-{next}");
                        let record = SessionConfigRecord::TypedOperation(
                            SessionLaneRecord::OperationStarted(serde_json::json!({
                                "id": operation_id,
                                "lane": lane,
                                "intent": {"kind": "compaction"},
                            })),
                        );
                        let Some((record_type, data)) = operation_record_parts(&record) else {
                            let _ = reply
                                .send(Err("compaction operation could not be encoded".to_owned()));
                            continue;
                        };
                        if let Err(error) = validate_session_lane_record(&state, record_type, data)
                        {
                            let _ = reply.send(Err(error));
                            continue;
                        }
                        reduce_operation_record(&mut state, record_type, data);
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(Ok(operation_id));
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
                            let exists = state.entries.iter().any(|entry| entry.id == *leaf_id)
                                || state
                                    .config_records
                                    .iter()
                                    .any(|entry| entry.id == *leaf_id);
                            if !exists {
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
                    Command::Fork { target_id, reply } => {
                        let fork = match state.fork_at_message(&target_id) {
                            Ok(fork) => fork,
                            Err(error) => {
                                let _ = reply.send(Err(error));
                                continue;
                            }
                        };
                        state = fork;
                        next_id = state
                            .entries
                            .iter()
                            .filter_map(|entry| entry.id.strip_prefix("entry-"))
                            .filter_map(|value| value.parse::<u64>().ok())
                            .max()
                            .unwrap_or(state.sequence)
                            .saturating_add(1);
                        rebuild_tool_result_reservations(&state, &mut tool_result_ids);
                        pending_tool_starts.clear();
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(Ok(()));
                    }
                    Command::SelectTree { target_id, reply } => {
                        if !state.entries.iter().any(|entry| entry.id == target_id) {
                            let _ =
                                reply.send(Err(format!("tree target does not exist: {target_id}")));
                            continue;
                        }
                        state.leaf_id = Some(target_id);
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(Ok(()));
                    }
                    Command::Undo { reply } => {
                        let target = match state.undo_target() {
                            Ok(target) => target,
                            Err(error) => {
                                let _ = reply.send(Err(error));
                                continue;
                            }
                        };
                        state.leaf_id = Some(target);
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(Ok(()));
                    }
                    Command::Import(imported, reply) => {
                        import_session_worker(
                            &mut state,
                            imported,
                            &mut next_id,
                            &mut tool_result_ids,
                            &mut pending_tool_starts,
                        );
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(());
                    }
                    Command::Reset(reply) => {
                        reset_session_worker(
                            &mut state,
                            &mut next_id,
                            &mut tool_result_ids,
                            &mut pending_tool_starts,
                        );
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
                    Command::PrepareAndBeginCompaction {
                        token_estimates,
                        keep_recent_tokens,
                        lane,
                        reply,
                    } => {
                        let preparation = match prepare_compaction_entries(
                            &state.entries,
                            &token_estimates,
                            keep_recent_tokens,
                        ) {
                            Ok(preparation) => preparation,
                            Err(error) => {
                                let _ = reply.send(Err(error));
                                continue;
                            }
                        };
                        let Some(preparation) = preparation else {
                            let _ = reply.send(Ok(None));
                            continue;
                        };
                        let next = state
                            .lane_records
                            .iter()
                            .filter_map(|record| record.id.strip_prefix("compaction-"))
                            .filter_map(|value| value.parse::<u64>().ok())
                            .max()
                            .unwrap_or_default()
                            .saturating_add(1);
                        let operation_id = format!("compaction-{next}");
                        let record = SessionConfigRecord::TypedOperation(
                            SessionLaneRecord::OperationStarted(serde_json::json!({
                                "id": operation_id,
                                "lane": lane,
                                "intent": {"kind": "compaction"},
                            })),
                        );
                        let Some((record_type, data)) = operation_record_parts(&record) else {
                            let _ = reply
                                .send(Err("compaction operation could not be encoded".to_owned()));
                            continue;
                        };
                        if let Err(error) = validate_session_lane_record(&state, record_type, data)
                        {
                            let _ = reply.send(Err(error));
                            continue;
                        }
                        reduce_operation_record(&mut state, record_type, data);
                        let entries = state.entries.clone();
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(Ok(Some((operation_id, preparation, entries))));
                    }
                    Command::PublishCompaction {
                        preparation,
                        summary,
                        reply,
                    } => {
                        let event = match summary.into_event(&preparation, &state.entries) {
                            Ok(event) => event,
                            Err(error) => {
                                let _ = reply.send(Err(error));
                                continue;
                            }
                        };
                        let AgentEvent::CompactionCreated {
                            summary,
                            retained_tail,
                            tokens_before,
                            details,
                            usage,
                        } = event
                        else {
                            unreachable!("compaction summary builder returned its fixed event");
                        };
                        state.sequence += 1;
                        let id = format!("entry-{next_id}");
                        next_id += 1;
                        let entry = SessionConfigEntry {
                            id: id.clone(),
                            lane: "main".into(),
                            seq: state.sequence,
                            parent_id: state.leaf_id.clone(),
                            timestamp: 0,
                            record: SessionConfigRecord::CompactionCreated {
                                summary,
                                retained_tail,
                                tokens_before,
                                details,
                                usage,
                            },
                        };
                        state.leaf_id = Some(id);
                        state.config_records.push(entry);
                        let _ = snapshot_tx.send(state.clone());
                        let _ = reply.send(Ok(()));
                    }
                }
            }
        }
    }};
}
