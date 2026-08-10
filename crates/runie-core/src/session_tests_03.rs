    async fn tool_started_actor() -> SessionActor {
        let actor = SessionActor::new();
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
                    name: "read".into(),
                    arguments: serde_json::json!({"path": "Cargo.toml"}),
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
                    "toolName": "read",
                    "effectiveArgs": {"path": "Cargo.toml"},
                    "resultEntryId": "entry-2",
                    "replay": "never"
                }),
            })
            .await;
        actor
    }

    #[tokio::test]
    async fn actor_reduces_complete_tool_started_identity() {
        let snapshot = tool_started_actor().await.snapshot();
        let record = snapshot
            .lane_records
            .iter()
            .find(|record| record.record_type == "tool_started")
            .expect("complete tool-start record");
        assert_eq!(record.data["assistantEntryId"], "entry-1");
        assert_eq!(record.data["resultEntryId"], "entry-2");
        assert_eq!(record.data["replay"], "never");
    }

    #[test]
    fn queue_lane_records_require_a_linked_provisioned_target() {
        let mut snapshot = SessionSnapshot::default();
        assert!(validate_session_lane_record(
            &snapshot,
            "queue_enqueued",
            &serde_json::json!({"id": "queue-1", "queue": "steer"})
        )
        .is_err());
        snapshot
            .active_operations
            .insert("run-1".into(), "started".into());
        let enqueue = serde_json::json!({
            "id": "queue-1",
            "runId": "run-1",
            "queue": "steer",
            "target": {"id": "entry-1", "role": "user", "content": "hello"}
        });
        assert!(validate_session_lane_record(&snapshot, "queue_enqueued", &enqueue).is_ok());
        snapshot.lane_records.push(SessionLaneRecordSnapshot {
            record_type: "queue_enqueued".into(),
            id: "queue-1".into(),
            lane: None,
            seq: None,
            timestamp: None,
            data: enqueue,
        });
        assert!(validate_session_lane_record(
            &snapshot,
            "queue_cancelled",
            &serde_json::json!({"id": "cancel-1", "runId": "run-1", "entryId": "entry-1"})
        )
        .is_ok());
        assert!(validate_session_lane_record(
            &snapshot,
            "queue_cancelled",
            &serde_json::json!({"id": "cancel-2", "runId": "run-2", "entryId": "entry-1"})
        )
        .is_err());
    }

    #[tokio::test]
    async fn navigation_operation_reduces_to_owned_intent_projection() {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        bus.publish(AgentEvent::OperationRecordCreated {
            record_type: "operation_started".into(),
            data: serde_json::json!({
                "id": "navigation-1",
                "intent": {
                    "kind": "navigation",
                    "targetId": "entry-target",
                    "summarize": true,
                    "summaryEntryId": "summary-target"
                }
            }),
        });
        actor.flush().await;
        assert_eq!(
            actor.snapshot().navigation,
            Some(NavigationSnapshot {
                target_id: Some("entry-target".into()),
                summarize: true,
                summary_entry_id: Some("summary-target".into()),
            })
        );
    }

    #[test]
    fn branch_entry_ids_follow_shared_parent_links() {
        let mut snapshot = SessionSnapshot::default();
        let message_entry = |seq: u64, parent_id: Option<&str>, id: &str| SessionEntry {
            id: id.into(),
            lane: "main".into(),
            seq,
            parent_id: parent_id.map(str::to_owned),
            timestamp: 0,
            message: user("test"),
            terminate: false,
        };
        snapshot.entries = vec![
            message_entry(1, None, "message-1"),
            message_entry(2, Some("message-1"), "message-2"),
        ];
        snapshot.config_records = vec![SessionConfigEntry {
            id: "config-3".into(),
            lane: "main".into(),
            seq: 3,
            parent_id: Some("message-2".into()),
            timestamp: 0,
            record: SessionConfigRecord::CustomSessionEntryCreated {
                custom_type: "test".into(),
                data: None,
            },
        }];
        snapshot.leaf_id = Some("config-3".into());
        assert_eq!(
            snapshot.branch_entry_ids(),
            ["message-1", "message-2", "config-3"]
        );
    }

    fn branch_snapshot() -> SessionSnapshot {
        SessionSnapshot {
            entries: vec![
                SessionEntry {
                    id: "message-1".into(),
                    lane: "main".into(),
                    seq: 1,
                    parent_id: None,
                    timestamp: 0,
                    message: user("one"),
                    terminate: false,
                },
                SessionEntry {
                    id: "message-2".into(),
                    lane: "feature".into(),
                    seq: 2,
                    parent_id: Some("message-1".into()),
                    timestamp: 0,
                    message: user("two"),
                    terminate: false,
                },
            ],
            config_records: vec![SessionConfigEntry {
                id: "config-1".into(),
                lane: "main".into(),
                seq: 3,
                parent_id: Some("message-2".into()),
                timestamp: 0,
                record: SessionConfigRecord::NameChanged {
                    name: "main".into(),
                },
            }],
            leaf_id: Some("message-2".into()),
            entry_lanes: BTreeMap::from([
                ("message-1".into(), "main".into()),
                ("message-2".into(), "feature".into()),
            ]),
            ..SessionSnapshot::default()
        }
    }

    #[test]
    fn branch_entry_query_requires_start_and_respects_stop_and_limit() {
        let snapshot = branch_snapshot();
        let entries = snapshot
            .find_entries_on_branch(&SessionBranchEntryQuery {
                start: "message-2".into(),
                stop_at_id: Some("message-1".into()),
                newest_first: true,
                limit: Some(1),
                ..SessionBranchEntryQuery::default()
            })
            .expect("branch query");
        assert!(
            matches!(entries[0], SessionEntryRecord::Message(ref entry) if entry.id == "message-2")
        );
        let feature_entries = snapshot
            .find_entries_on_branch(&SessionBranchEntryQuery {
                start: "config-1".into(),
                lane: Some("feature".into()),
                ..SessionBranchEntryQuery::default()
            })
            .expect("lane branch query");
        assert_eq!(
            feature_entries
                .iter()
                .filter_map(|entry| match entry {
                    SessionEntryRecord::Message(entry) => Some(entry.id.as_str()),
                    SessionEntryRecord::Config(_) => None,
                })
                .collect::<Vec<_>>(),
            vec!["message-2"]
        );
        assert!(snapshot
            .find_entries_on_branch(&SessionBranchEntryQuery {
                start: "missing".into(),
                ..SessionBranchEntryQuery::default()
            })
            .is_err());
    }

    #[test]
    fn singular_entry_queries_preserve_declared_order() {
        let snapshot = SessionSnapshot {
            entries: vec![SessionEntry {
                id: "message-1".into(),
                lane: "main".into(),
                seq: 1,
                parent_id: None,
                timestamp: 0,
                message: user("one"),
                terminate: false,
            }],
            leaf_id: Some("message-1".into()),
            ..SessionSnapshot::default()
        };
        assert!(matches!(
            snapshot.find_entry(&SessionEntryQuery::default()),
            Some(SessionEntryRecord::Message(entry)) if entry.id == "message-1"
        ));
        assert!(matches!(
            snapshot
                .find_entry_on_branch(&SessionBranchEntryQuery {
                    start: "message-1".into(),
                    ..SessionBranchEntryQuery::default()
                })
                .expect("branch lookup"),
            Some(SessionEntryRecord::Message(entry)) if entry.id == "message-1"
        ));
    }

    fn fork_snapshot() -> SessionSnapshot {
        SessionSnapshot {
            entries: vec![
                SessionEntry {
                    id: "message-1".into(),
                    lane: "main".into(),
                    seq: 1,
                    parent_id: None,
                    timestamp: 0,
                    message: user("one"),
                    terminate: false,
                },
                SessionEntry {
                    id: "message-2".into(),
                    lane: "main".into(),
                    seq: 2,
                    parent_id: Some("message-1".into()),
                    timestamp: 0,
                    message: user("two"),
                    terminate: false,
                },
                SessionEntry {
                    id: "message-3".into(),
                    lane: "main".into(),
                    seq: 3,
                    parent_id: Some("message-2".into()),
                    timestamp: 0,
                    message: user("three"),
                    terminate: false,
                },
            ],
            leaf_id: Some("message-3".into()),
            ..SessionSnapshot::default()
        }
    }

    #[test]
    fn fork_at_message_resequences_only_the_validated_branch_prefix() {
        let snapshot = fork_snapshot();
        let fork = snapshot.fork_at_message("message-2").expect("fork");
        assert_eq!(fork.sequence, 3);
        assert_eq!(fork.leaf_id.as_deref(), Some("message-2"));
        assert_eq!(
            fork.entries
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            ["message-1", "message-2"]
        );
        assert_eq!(
            fork.entries
                .iter()
                .map(|entry| entry.seq)
                .collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(fork.lanes().get("main"), Some(&Some("message-2".into())));
        assert!(snapshot.fork_at_message("missing").is_err());
    }
    #[test]
    fn compaction_cut_point_preserves_recent_budget_and_reports_split_turn() {
        let entries = vec![
            SessionEntry {
                id: "user-1".into(),
                lane: "main".into(),
                seq: 1,
                parent_id: None,
                timestamp: 0,
                message: user("request"),
                terminate: false,
            },
            SessionEntry {
                id: "assistant-1".into(),
                lane: "main".into(),
                seq: 2,
                parent_id: Some("user-1".into()),
                timestamp: 0,
                message: AgentMessage::Assistant(Default::default()),
                terminate: false,
            },
        ];
        let cut = find_compaction_cut_point(&entries, &[40, 40], 0, 2, 20).expect("cut point");
        assert_eq!(cut.first_kept_entry_index, 1);
        assert_eq!(cut.turn_start_index, Some(0));
        assert!(cut.is_split_turn);
    }

    #[test]
    fn compaction_threshold_matches_pi_enabled_and_strict_boundary() {
        let settings = CompactionSettings {
            enabled: true,
            reserve_tokens: 100,
            keep_recent_tokens: 20,
        };
        assert!(!should_compact(900, 1_000, settings));
        assert!(!should_compact(100, 1_000, settings));
        assert!(should_compact(901, 1_000, settings));
        assert!(!should_compact(
            901,
            1_000,
            CompactionSettings {
                enabled: false,
                ..settings
            }
        ));
        assert!(should_compact(
            u64::MAX,
            10,
            CompactionSettings {
                reserve_tokens: u64::MAX,
                ..settings
            }
        ));
    }
