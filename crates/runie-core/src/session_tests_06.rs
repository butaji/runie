    fn configuration_snapshot() -> SessionSnapshot {
        SessionSnapshot {
            sequence: 2,
            leaf_id: Some("entry-2".into()),
            entries: Vec::new(),
            entry_lanes: BTreeMap::new(),
            config_records: vec![
                SessionConfigEntry {
                    id: "entry-1".into(),
                    lane: "main".into(),
                    seq: 1,
                    parent_id: None,
                    timestamp: 0,
                    record: SessionConfigRecord::ThinkingLevelChanged {
                        level: crate::types::ThinkingLevel::High,
                    },
                },
                SessionConfigEntry {
                    id: "entry-2".into(),
                    lane: "main".into(),
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
        }
    }

    #[test]
    fn jsonl_round_trip_preserves_configuration_records() {
        let snapshot = configuration_snapshot();
        let jsonl = snapshot.to_jsonl("session-1", 5, "/workspace");
        assert!(jsonl.contains("\"type\":\"thinking_level_change\""));
        let (_, _, imported) = SessionSnapshot::from_jsonl(&jsonl).expect("valid JSONL");
        assert_eq!(imported.config_records, snapshot.config_records);
    }

    #[tokio::test]
    async fn user_question_trace_is_a_durable_custom_session_record() {
        let actor = SessionActor::new();
        actor
            .record_user_question_trace(crate::tools::UserQuestionTrace {
                id: "question-1".into(),
                question: "Continue?".into(),
                outcome: "answered".into(),
                attempted_answer: None,
                error: None,
            })
            .await
            .expect("question trace record");
        let jsonl = actor.snapshot().to_jsonl("session-1", 5, "/workspace");
        assert!(jsonl.contains("user_question_trace"));
        let (_, _, restored) = SessionSnapshot::from_jsonl(&jsonl).expect("trace restore");
        assert!(restored.config_records.iter().any(|entry| {
            matches!(
                &entry.record,
                SessionConfigRecord::CustomSessionEntryCreated { custom_type, .. }
                    if custom_type == "user_question_trace"
            )
        }));
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

    async fn unsettled_tool_jsonl() -> String {
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
        source.snapshot().to_jsonl("session-1", 5, "/workspace")
    }

    #[tokio::test]
    async fn actor_restore_rebuilds_unsettled_tool_result_reservation() {
        let jsonl = unsettled_tool_jsonl().await;

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
            entry(1, serde_json::Value::Null, "not-entry")
        );
        assert!(SessionSnapshot::from_jsonl(&unsupported_kind).is_err());
    }

    #[test]
    fn jsonl_round_trips_opaque_extension_records() {
        let header = serde_json::json!({
            "kind": "header", "version": 4, "id": "s", "createdAt": 5, "cwd": "/w"
        })
        .to_string();
        let extension = serde_json::json!({
            "kind": "entry", "lane": "secondary", "type": "plugin_event",
            "id": "plugin-1", "parentId": null, "seq": 1, "timestamp": 7,
            "plugin": "example", "payload": {"enabled": true}
        });
        let (_, _, snapshot) = SessionSnapshot::from_jsonl(&format!("{header}\n{}\n", extension))
            .expect("opaque extension record");
        assert!(matches!(
            snapshot.config_records.first().map(|entry| &entry.record),
            Some(SessionConfigRecord::OperationRecordCreated { record_type, data })
                if record_type == "plugin_event" && data["payload"]["enabled"] == true
        ));
        assert_eq!(snapshot.config_records[0].lane, "secondary");
        let exported = snapshot.to_jsonl("s", 5, "/w");
        assert!(exported.contains("\"type\":\"plugin_event\""));
        assert!(exported.contains("\"lane\":\"secondary\""));
        assert!(exported.contains("\"enabled\":true"));
    }

    #[test]
    fn jsonl_import_rejects_operation_records_without_ordered_storage_metadata() {
        let header = serde_json::json!({
            "kind": "header", "version": 4, "id": "s", "createdAt": 5, "cwd": "/w"
        })
        .to_string();
        let valid = serde_json::json!({
            "kind": "entry", "lane": "main", "type": "operation_started",
            "id": "run-1", "seq": 1, "timestamp": 7
        });
        assert!(SessionSnapshot::from_jsonl(&format!("{header}\n{valid}\n")).is_ok());

        let missing_seq = serde_json::json!({
            "kind": "entry", "lane": "main", "type": "operation_started",
            "id": "run-1", "timestamp": 7
        });
        assert!(SessionSnapshot::from_jsonl(&format!("{header}\n{missing_seq}\n")).is_err());

        let wrong_order = serde_json::json!({
            "kind": "entry", "lane": "main", "type": "operation_started",
            "id": "run-1", "seq": 0, "timestamp": 7
        });
        assert!(SessionSnapshot::from_jsonl(&format!("{header}\n{wrong_order}\n")).is_err());

        let empty_lane = serde_json::json!({
            "kind": "lane", "lane": "", "seq": 1, "leafId": null
        });
        assert!(SessionSnapshot::from_jsonl(&format!("{header}\n{empty_lane}\n")).is_err());
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
