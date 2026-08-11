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
                lane: "main".into(),
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
                lane: "main".into(),
                seq: 1,
                parent_id: None,
                timestamp: 7,
                message: user("hello"),
                terminate: false,
            }],
            config_records: vec![SessionConfigEntry {
                id: "entry-2".into(),
                lane: "main".into(),
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

    fn usage_snapshot() -> SessionSnapshot {
        let mut snapshot = SessionSnapshot {
            entries: vec![SessionEntry {
                id: "entry-1".into(),
                lane: "main".into(),
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
        snapshot
    }

    #[test]
    fn session_stats_reduce_usage_records_like_pi() {
        assert_eq!(
            usage_snapshot().stats(),
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
    fn session_stats_round_trip_as_replay_data() {
        let stats = usage_snapshot().stats();
        let encoded = serde_json::to_string(&stats).unwrap();
        let decoded: SessionStats = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, stats);
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
fn lane_sequence_projection_rejects_reordered_or_future_records() {
    let mut snapshot = SessionSnapshot {
        sequence: 3,
        ..SessionSnapshot::default()
    };
    snapshot.lane_records = vec![
        SessionLaneRecordSnapshot {
            record_type: "usage".into(),
            id: "run-1".into(),
            lane: Some("main".into()),
            seq: Some(1),
            timestamp: Some(1),
            data: serde_json::json!({}),
        },
        SessionLaneRecordSnapshot {
            record_type: "usage".into(),
            id: "run-1".into(),
            lane: Some("main".into()),
            seq: Some(3),
            timestamp: Some(2),
            data: serde_json::json!({}),
        },
    ];
    assert!(snapshot.validate_lane_sequences().is_ok());
    snapshot.lane_records[1].seq = Some(1);
    assert!(snapshot.validate_lane_sequences().is_err());
}

    fn lane_record_is_valid(snapshot: &SessionSnapshot, kind: &str, data: serde_json::Value) -> bool {
        validate_session_lane_record(snapshot, kind, &data).is_ok()
    }

    fn lane_record_is_invalid(snapshot: &SessionSnapshot, kind: &str, data: serde_json::Value) -> bool {
        !lane_record_is_valid(snapshot, kind, data)
    }

    fn tool_record_valid(snapshot: &SessionSnapshot, data: serde_json::Value) -> bool {
        validate_tool_started_record(snapshot, SessionLaneRecordKind::ToolStarted, &data).is_ok()
    }

    #[test]
    fn duplicate_or_malformed_lane_records_are_rejected_purely() {
        let mut snapshot = SessionSnapshot::default();
        assert!(lane_record_is_valid(&snapshot, "operation_started", serde_json::json!({"id":"op-1","lane":"main","intent":{"kind":"run"}})));
        snapshot
            .active_operations
            .insert("op-1".into(), "started".into());
        assert!(lane_record_is_invalid(&snapshot, "operation_started", serde_json::json!({"id":"op-1","lane":"main","intent":{"kind":"run"}})));
        assert!(lane_record_is_invalid(&SessionSnapshot::default(), "operation_started", serde_json::json!({"id":"unknown-kind","lane":"main","intent":{"kind":"workflow"}})));
        assert!(lane_record_is_invalid(&snapshot, "operation_finished", serde_json::json!({"outcome":"completed"})));
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
            lane: "main".into(),
            terminate: false,
        });
        let valid = serde_json::json!({
            "assistantEntryId": "assistant-1", "toolIndex": 0,
            "toolCallId": "call-1", "toolName": "read",
            "resultEntryId": "result-1", "replay": "never"
        });
        assert!(tool_record_valid(&snapshot, valid));
        assert!(!tool_record_valid(&snapshot, serde_json::json!({"assistantEntryId":"assistant-1","toolIndex":1,"toolCallId":"call-1","toolName":"read","resultEntryId":"result-1","replay":"never"})));
    }
