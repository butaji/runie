    #[tokio::test]
    async fn compaction_identity_is_allocated_inside_the_session_actor() {
        let actor = SessionActor::new();
        let first = actor
            .begin_compaction("main".into())
            .await
            .expect("first compaction start");
        assert_eq!(first, "compaction-1");
        assert_eq!(
            actor.snapshot().active_operations.get(&first),
            Some(&"started".to_owned())
        );
        assert_eq!(
            actor.snapshot().operation_kinds.get(&first),
            Some(&"compaction".to_owned())
        );

        actor
            .record_operation(
                SessionOperationKind::Finished,
                serde_json::json!({"id": first, "outcome": "completed"}),
            )
            .await
            .expect("finish first compaction");
        let second = actor
            .begin_compaction("main".into())
            .await
            .expect("second compaction start");
        assert_eq!(second, "compaction-2");
    }

    #[allow(
        clippy::too_many_lines,
        reason = "storage round-trip covers publish, load, and fork"
    )]
    #[tokio::test]
    async fn storage_actor_publishes_a_complete_jsonl_file_atomically() {
        let path = std::env::temp_dir().join(format!(
            "runie-session-{}-{}.jsonl",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let path_string = path.to_string_lossy().into_owned();
        let storage = SessionStorageActor::new();
        let snapshot = storage_snapshot();
        storage
            .publish_snapshot(&path_string, &snapshot, "session-1", 1, "/tmp")
            .await
            .expect("publish");
        let contents = tokio::fs::read_to_string(&path).await.expect("read");
        let header: serde_json::Value =
            serde_json::from_str(contents.lines().next().expect("header line"))
                .expect("header JSON");
        assert_eq!(header["kind"], "header");
        assert_eq!(header["version"], 4);
        let (session_id, cwd, loaded) = storage.load_snapshot(&path_string).await.expect("load");
        assert_eq!(session_id, "session-1");
        assert_eq!(cwd, "/tmp");
        assert_eq!(loaded.sequence, snapshot.sequence);
        tokio::fs::write(&path, format!("{contents}{{\"kind\":\"entry\""))
            .await
            .expect("tear final line");
        let (_, _, repaired) = storage
            .load_snapshot(&path_string)
            .await
            .expect("repair load");
        assert_eq!(repaired.sequence, snapshot.sequence);
        let fork_path = format!("{path_string}.fork");
        storage
            .fork_snapshot(&fork_path, &snapshot, "entry-1", "fork-1", 2, "/tmp")
            .await
            .expect("fork publish");
        let (_, _, forked) = storage.load_snapshot(&fork_path).await.expect("fork load");
        assert_eq!(forked.sequence, 2);
        assert_eq!(forked.leaf_id.as_deref(), Some("entry-1"));
        let feature_snapshot = feature_fork_snapshot(&snapshot);
        let feature_fork_path = format!("{path_string}.feature-fork");
        storage
            .fork_snapshot_in_lane(
                &feature_fork_path,
                &feature_snapshot,
                Some("feature"),
                "entry-2",
                "fork-feature",
                3,
                "/tmp",
            )
            .await
            .expect("feature-lane fork publish");
        let (_, _, feature_forked) = storage
            .load_snapshot(&feature_fork_path)
            .await
            .expect("feature-lane fork load");
        assert_eq!(feature_forked.entry_lane("entry-2"), Some("feature"));
        assert_eq!(feature_forked.entries.len(), 2);
        assert!(!tokio::fs::try_exists(format!("{path_string}.tmp"))
            .await
            .expect("temporary file check"));
        let _ = tokio::fs::remove_file(path).await;
        let _ = tokio::fs::remove_file(fork_path).await;
        let _ = tokio::fs::remove_file(feature_fork_path).await;
    }

    #[tokio::test]
    async fn storage_discovery_is_bounded_sorted_and_header_validated() {
        let root = std::env::temp_dir().join(format!("runie-discover-{}", std::process::id()));
        let _ = tokio::fs::remove_dir_all(&root).await;
        tokio::fs::create_dir_all(&root).await.expect("directory");
        let storage = SessionStorageActor::new();
        let snapshot = storage_snapshot();
        storage
            .publish_snapshot(root.join("b.jsonl").to_string_lossy(), &snapshot, "b", 2, "/b")
            .await
            .expect("publish b");
        storage
            .publish_snapshot(root.join("a.jsonl").to_string_lossy(), &snapshot, "a", 1, "/a")
            .await
            .expect("publish a");
        tokio::fs::write(root.join("invalid.jsonl"), "not-json\n")
            .await
            .expect("invalid fixture");
        let found = storage.discover(root.to_string_lossy(), 1).await.expect("discover");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].session_id, "a");
        assert_eq!(found[0].cwd, "/a");
        let _ = tokio::fs::remove_dir_all(root).await;
    }

    #[tokio::test]
    async fn storage_discovery_rows_are_queryable_at_the_actor_boundary() {
        let root = std::env::temp_dir().join(format!("runie-discover-rows-{}", std::process::id()));
        let _ = tokio::fs::remove_dir_all(&root).await;
        tokio::fs::create_dir_all(&root).await.expect("directory");
        let storage = SessionStorageActor::new();
        storage
            .publish_snapshot(root.join("deploy.jsonl").to_string_lossy(), &storage_snapshot(), "deploy", 1, "/work/release")
            .await
            .expect("publish");
        let rows = storage
            .discover_rows(root.to_string_lossy(), 8, "release")
            .await
            .expect("discover rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].session_id, "deploy");
        let _ = tokio::fs::remove_dir_all(root).await;
    }

    #[test]
    fn storage_entries_project_to_serializable_picker_rows() {
        let entry = SessionStorageEntry {
            path: "/sessions/abc.jsonl".into(),
            session_id: "abc".into(),
            created_at: 42,
            cwd: "/workspace/runie".into(),
        };

        assert_eq!(
            entry.row(),
            SessionStorageRow {
                session_id: "abc".into(),
                label: "runie".into(),
                cwd: "/workspace/runie".into(),
                created_at: 42,
            }
        );
        assert!(serde_json::to_value(entry.row()).is_ok());
    }

    #[test]
    fn navigation_validation_checks_target_and_summary_ids() {
        let mut snapshot = SessionSnapshot {
            navigation: Some(NavigationSnapshot {
                target_id: Some("entry-1".into()),
                summarize: true,
                summary_entry_id: Some("summary-1".into()),
            }),
            ..SessionSnapshot::default()
        };
        snapshot.entries.push(SessionEntry {
            id: "entry-1".into(),
            lane: "main".into(),
            seq: 1,
            parent_id: None,
            timestamp: 0,
            message: user("target"),
            terminate: false,
        });
        snapshot.config_records.push(SessionConfigEntry {
            id: "summary-1".into(),
            lane: "main".into(),
            seq: 2,
            parent_id: Some("entry-1".into()),
            timestamp: 0,
            record: SessionConfigRecord::BranchSummaryCreated {
                from_id: "entry-1".into(),
                summary: "summary".into(),
                details: None,
            },
        });
        assert_eq!(
            snapshot.navigation_validation(),
            Some(NavigationValidation {
                target_exists: true,
                summary_exists: true,
            })
        );
    }

    #[tokio::test]
    #[allow(
        clippy::too_many_lines,
        reason = "integration fixture spells out tool lifecycle ordering"
    )]
    async fn bus_tool_termination_is_attached_to_the_owned_session_entry() {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        let _ = actor
            .record_config(SessionConfigRecord::OperationRecordCreated {
                record_type: "operation_started".into(),
                data: serde_json::json!({"id": "run-1"}),
            })
            .await;
        actor
            .append(AgentMessage::Assistant(AssistantMessage {
                content: vec![AssistantContent::ToolCall(ToolCall {
                    id: "call-1".into(),
                    name: "stop".into(),
                    arguments: serde_json::json!({"reason": "test"}),
                    thought_signature: None,
                })],
                ..Default::default()
            }))
            .await;
        let _ = actor
            .record_config(SessionConfigRecord::OperationRecordCreated {
                record_type: "tool_started".into(),
                data: serde_json::json!({
                    "runId": "run-1",
                    "assistantEntryId": "entry-1",
                    "toolIndex": 0,
                    "toolCallId": "call-1",
                    "toolName": "stop",
                    "effectiveArgs": {"reason": "test"},
                    "resultEntryId": "entry-2",
                    "replay": "never"
                }),
            })
            .await;
        bus.publish(AgentEvent::ToolExecutionEnd {
            tool_call_id: "call-1".into(),
            tool_name: "stop".into(),
            result: serde_json::json!({"terminate": true}),
            is_error: false,
        });
        bus.publish(AgentEvent::MessageEnd {
            message: AgentMessage::ToolResult(ToolResultMessage {
                tool_call_id: "call-1".into(),
                tool_name: "stop".into(),
                content: vec![ToolResultContent::Text {
                    text: "done".into(),
                }],
                ..Default::default()
            }),
        });
        actor.flush().await;
        assert_eq!(actor.snapshot().lane_records[3].record_type, "tool_started");
        assert_eq!(actor.snapshot().entries.len(), 2);
    }

    #[tokio::test]
    async fn assistant_message_end_emits_owned_usage_lane_record() {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        bus.publish(AgentEvent::OperationRecordCreated {
            record_type: "operation_started".into(),
            data: serde_json::json!({"id": "run-1", "lane": "main", "runId": "run-1"}),
        });
        bus.publish(AgentEvent::OperationRecordCreated {
            record_type: "operation_started".into(),
            data: serde_json::json!({"id": "run-2", "lane": "main", "runId": "run-2"}),
        });
        let assistant = AssistantMessage {
            usage: Usage::default(),
            ..Default::default()
        };
        bus.publish(AgentEvent::MessageEnd {
            message: AgentMessage::Assistant(assistant),
        });
        actor.flush().await;
        let snapshot = actor.snapshot();
        assert_eq!(
            snapshot
                .lane_records
                .iter()
                .map(|record| record.record_type.as_str())
                .collect::<Vec<_>>(),
            [
                "operation_started",
                "operation_started",
                "step_attempt",
                "usage"
            ]
        );
        assert_eq!(snapshot.lane_records[2].data["runId"], "run-2");
        assert_eq!(snapshot.lane_records[2].data["resultEntryId"], "entry-1");
        assert_eq!(snapshot.lane_records[3].id, "entry-1");
    }

    #[tokio::test]
    async fn deferred_assistant_commit_emits_write_deferred_lane_record() {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        let assistant = AssistantMessage {
            stop_reason: Some(StopReason::Deferred),
            deferred: Some(DeferredHandle {
                provider: "replay".into(),
                model_id: "model-1".into(),
                api: "replay-api".into(),
                id: "deferred-1".into(),
                expires_at: None,
                poll_after_ms: None,
                data: None,
            }),
            ..Default::default()
        };
        bus.publish(AgentEvent::MessageEnd {
            message: AgentMessage::Assistant(assistant),
        });
        actor.flush().await;
        assert_eq!(
            actor.snapshot().lane_records[1].record_type,
            "write_deferred"
        );
        assert_eq!(actor.snapshot().lane_records[1].id, "entry-1");
    }

    #[tokio::test]
    async fn snapshot_exports_pi_jsonl_v4_header_and_parented_message_entry() {
        let actor = SessionActor::new();
        actor.append(user("one")).await;
        actor.append(user("two")).await;
        let lines = actor.snapshot().to_jsonl("session-1", 5, "/workspace");
        let values = lines
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("jsonl line"))
            .collect::<Vec<_>>();
        assert_eq!(values[0]["kind"], "header");
        assert_eq!(values[0]["version"], 4);
        assert_eq!(values[1]["type"], "message");
        assert_eq!(values[1]["parentId"], serde_json::Value::Null);
        assert_eq!(values[2]["parentId"], "entry-1");
        assert_eq!(values[2]["seq"], 2);
    }

    #[test]
    fn jsonl_round_trip_preserves_pi_terminate_entry_metadata() {
        let snapshot = SessionSnapshot {
            sequence: 1,
            leaf_id: Some("entry-1".into()),
            entries: vec![SessionEntry {
                id: "entry-1".into(),
                lane: "main".into(),
                seq: 1,
                parent_id: None,
                timestamp: 7,
                message: user("stop here"),
                terminate: true,
            }],
            entry_lanes: BTreeMap::from([(String::from("entry-1"), String::from("main"))]),
            config_records: Vec::new(),
            lane_facts: Vec::new(),
            lane_records: Vec::new(),
            active_operations: BTreeMap::new(),
            operation_outcomes: BTreeMap::new(),
            operation_kinds: BTreeMap::new(),
            operation_errors: BTreeMap::new(),
            navigation: None,
        };
        let jsonl = snapshot.to_jsonl("session-1", 5, "/workspace");
        assert!(jsonl.contains("\"terminate\":true"));
        let (_, _, imported) = SessionSnapshot::from_jsonl(&jsonl).expect("valid JSONL");
        assert!(imported.entries[0].terminate);
    }
fn storage_snapshot() -> SessionSnapshot {
    SessionSnapshot {
        sequence: 1,
        leaf_id: Some("entry-1".into()),
        entries: vec![SessionEntry {
            id: "entry-1".into(),
            lane: "main".into(),
            seq: 1,
            parent_id: None,
            timestamp: 0,
            message: user("fork me"),
            terminate: false,
        }],
        ..SessionSnapshot::default()
    }
}

fn feature_fork_snapshot(snapshot: &SessionSnapshot) -> SessionSnapshot {
    SessionSnapshot {
        sequence: 2,
        leaf_id: Some("entry-1".into()),
        entries: vec![
            snapshot.entries[0].clone(),
            SessionEntry {
                id: "entry-2".into(),
                lane: "feature".into(),
                seq: 2,
                parent_id: Some("entry-1".into()),
                timestamp: 0,
                message: user("feature fork me"),
                terminate: false,
            },
        ],
        ..snapshot.clone()
    }
}
