    #[tokio::test]
    async fn typed_operation_api_publishes_the_internal_lane_union() {
        let actor = SessionActor::new();
        actor
            .record_typed_operation(SessionLaneRecord::OperationStarted(
                serde_json::json!({"id": "typed-run", "intent": {"kind": "run"}}),
            ))
            .await
            .expect("typed operation admission");

        let snapshot = actor.snapshot();
        assert_eq!(snapshot.lane_records.len(), 1);
        assert_eq!(snapshot.lane_records[0].record_type, "operation_started");
        assert_eq!(
            snapshot.lane_records[0]
                .typed_record()
                .expect("typed record"),
            SessionLaneRecord::OperationStarted(serde_json::json!({
                "id": "typed-run",
                "intent": {"kind": "run"}
            }))
        );
    }

    #[tokio::test]
    async fn bus_lane_admission_rejection_is_published_as_an_error_event() {
        let bus = EventBus::new();
        let mut events = bus.subscribe();
        let _actor = SessionActor::new_with_bus(&bus);
        bus.publish(AgentEvent::TypedOperationRecordCreated {
            kind: crate::types::OperationRecordKind::StepAttempt,
            data: serde_json::json!({"runId": "missing", "step": "assistant"}),
        });

        let rejection = tokio::time::timeout(std::time::Duration::from_secs(1), async {
            loop {
                if let AgentEvent::Error { message } = events.recv().await.expect("event bus") {
                    break message;
                }
            }
        })
        .await
        .expect("session rejection event");
        assert!(rejection.starts_with("Session event rejected:"));
    }

    #[tokio::test]
    async fn bus_configuration_events_reduce_to_ordered_session_records() {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        bus.publish(AgentEvent::ModelChanged {
            model: crate::types::Model {
                id: "model-1".into(),
                provider: "provider-1".into(),
                ..Default::default()
            },
        });
        bus.publish(AgentEvent::ThinkingLevelChanged {
            level: crate::types::ThinkingLevel::High,
        });
        actor.flush().await;
        let records = actor.snapshot().config_records;
        assert_eq!(records.len(), 2);
        assert!(matches!(
            records[0].record,
            SessionConfigRecord::ModelChanged { ref provider, ref model_id }
                if provider == "provider-1" && model_id == "model-1"
        ));
        assert!(matches!(
            records[1].record,
            SessionConfigRecord::ThinkingLevelChanged {
                level: crate::types::ThinkingLevel::High
            }
        ));
        assert_eq!(records[0].seq, 1);
        assert_eq!(records[1].parent_id.as_deref(), Some("entry-1"));
    }

    async fn started_operation_actor() -> (EventBus, SessionActor) {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        bus.publish(AgentEvent::OperationRecordCreated {
            record_type: "operation_started".into(),
            data: serde_json::json!({"id": "op-1"}),
        });
        actor.flush().await;
        (bus, actor)
    }

    async fn complete_operation(bus: &EventBus, actor: &SessionActor) {
        bus.publish(AgentEvent::OperationRecordCreated { record_type: "abort_requested".into(), data: serde_json::json!({"id":"op-1"}) });
        actor.flush().await;
        assert_eq!(actor.snapshot().active_operations["op-1"], "aborted");
        bus.publish(AgentEvent::OperationRecordCreated { record_type: "operation_finished".into(), data: serde_json::json!({"id":"op-1","outcome":"aborted"}) });
        actor.flush().await;
        assert!(actor.snapshot().active_operations.is_empty());
        assert_eq!(actor.snapshot().operation_outcomes["op-1"], "aborted");
        assert_eq!(actor.snapshot().lane_records.iter().map(|record| record.record_type.as_str()).collect::<Vec<_>>(), vec!["operation_started", "abort_requested", "operation_finished"]);
    }

    fn assert_operation_json(original: &SessionSnapshot) {
        let jsonl = original.to_jsonl("session-ops", 5, "/workspace");
        let (_, _, imported) = SessionSnapshot::from_jsonl(&jsonl).expect("operation JSONL");
        assert_eq!(imported.active_operations, original.active_operations);
        assert_eq!(imported.operation_outcomes, original.operation_outcomes);
        assert!(imported.config_records.iter().all(|entry| !matches!(entry.record, SessionConfigRecord::OperationRecordCreated { .. })));
        assert_eq!(imported.lane_records.iter().map(|record| record.record_type.as_str()).collect::<Vec<_>>(), vec!["operation_started", "abort_requested", "operation_finished"]);
        assert_eq!(imported.typed_lane_records().expect("admitted records decode").into_iter().map(|(_, record)| record.kind()).collect::<Vec<_>>(), vec![SessionLaneRecordKind::OperationStarted, SessionLaneRecordKind::AbortRequested, SessionLaneRecordKind::OperationFinished]);
    }

    #[tokio::test]
    async fn operation_records_reduce_to_owned_lifecycle_state() {
        let (bus, actor) = started_operation_actor().await;
        assert_eq!(actor.snapshot().active_operations["op-1"], "started");
        assert_eq!(actor.snapshot().lane_records[0].data, serde_json::json!({"id":"op-1"}));
        let operation_event = AgentEvent::OperationRecordCreated {
            record_type: "operation_started".into(),
            data: serde_json::json!({"id": "op-1"}),
        };
        let typed = session_config_record!(&operation_event)
            .expect("known operation event is typed at the session boundary");
        assert!(matches!(
            typed,
            SessionConfigRecord::TypedOperation(SessionLaneRecord::OperationStarted(data))
                if data == serde_json::json!({"id": "op-1"})
        ));
        let before_rejected = actor.snapshot().lane_records.len();
        let rejected = actor
            .record_operation(
                SessionOperationKind::Started,
                serde_json::json!({"id": "op-1"}),
            )
            .await;
        assert!(rejected.is_err());
        assert_eq!(actor.snapshot().lane_records.len(), before_rejected);
        complete_operation(&bus, &actor).await;
        assert_operation_json(&actor.snapshot());
    }
    #[test]
    fn session_lane_record_validation_classifies_pi_families() {
        assert_eq!(
            session_lane_record_kind("tool_started"),
            Some(SessionLaneRecordKind::ToolStarted)
        );
        assert_eq!(
            validate_session_lane_record(
                &SessionSnapshot::default(),
                "usage",
                &serde_json::json!({"entryId": "entry-1"})
            ),
            Ok(SessionLaneRecordKind::Usage)
        );
        assert!(validate_session_lane_record(
            &SessionSnapshot::default(),
            "unknown",
            &serde_json::json!({"id": "record-1"})
        )
        .is_err());
    }

    #[test]
    fn typed_lane_record_decode_preserves_family_and_payload() {
        let payload = serde_json::json!({"runId": "op-1", "seq": 3});
        let record =
            SessionLaneRecord::decode("tool_started", &payload).expect("known Pi lane family");
        assert_eq!(record.kind(), SessionLaneRecordKind::ToolStarted);
        assert_eq!(record.identity(), Some("op-1"));
        assert_eq!(record.run_id(), Some("op-1"));
        assert!(matches!(record, SessionLaneRecord::ToolStarted(value) if value == payload));
        assert!(SessionLaneRecord::decode("unknown", &payload).is_err());
    }

    #[test]
    fn lane_snapshot_exposes_one_validated_typed_boundary() {
        let payload = serde_json::json!({"id": "op-1", "intent": {"kind": "prompt"}});
        let snapshot = SessionLaneRecordSnapshot {
            record_type: "operation_started".into(),
            id: "op-1".into(),
            lane: Some("operation".into()),
            seq: Some(1),
            timestamp: Some(0),
            data: payload.clone(),
        };

        assert_eq!(
            snapshot.kind().expect("known typed lane family"),
            SessionLaneRecordKind::OperationStarted
        );
        assert!(matches!(
            snapshot.typed_record().expect("valid snapshot payload"),
            SessionLaneRecord::OperationStarted(value) if value == payload
        ));
    }

    #[test]
    fn lane_snapshot_preserves_unknown_extensions_as_opaque_records() {
        let snapshot = SessionLaneRecordSnapshot {
            record_type: "plugin_extension".into(),
            id: "extension-1".into(),
            lane: Some("main".into()),
            seq: Some(2),
            timestamp: Some(1),
            data: serde_json::json!({"custom": true}),
        };

        assert!(matches!(
            snapshot.lossless_record(),
            SessionLaneRecordEnvelope::Opaque { ref record_type, ref data }
                if record_type == "plugin_extension" && data == &serde_json::json!({"custom": true})
        ));
    }

    #[test]
    fn session_snapshot_exposes_known_and_opaque_lane_records_losslessly() {
        let snapshot = SessionSnapshot {
            lane_records: vec![
                SessionLaneRecordSnapshot {
                    record_type: "usage".into(),
                    id: "entry-1".into(),
                    lane: Some("main".into()),
                    seq: Some(1),
                    timestamp: None,
                    data: serde_json::json!({"entryId": "entry-1"}),
                },
                SessionLaneRecordSnapshot {
                    record_type: "plugin_extension".into(),
                    id: "extension-1".into(),
                    lane: Some("main".into()),
                    seq: Some(2),
                    timestamp: None,
                    data: serde_json::json!({"custom": true}),
                },
            ],
            ..SessionSnapshot::default()
        };
        let records = snapshot.lossless_lane_records();
        assert!(matches!(
            records[0].1,
            SessionLaneRecordEnvelope::Known(SessionLaneRecord::Usage(_))
        ));
        assert!(matches!(
            records[1].1,
            SessionLaneRecordEnvelope::Opaque { .. }
        ));
    }

    #[test]
    fn typed_lane_identity_prefers_pi_record_shapes() {
        let entry = SessionLaneRecord::decode(
            "usage",
            &serde_json::json!({"entryId": "entry-1", "runId": "run-1"}),
        )
        .expect("usage record");
        assert_eq!(entry.identity(), Some("run-1"));
        assert_eq!(entry.run_id(), Some("run-1"));

        let queue = SessionLaneRecord::decode(
            "queue_cancelled",
            &serde_json::json!({"entryId": "entry-1"}),
        )
        .expect("queue cancellation");
        assert_eq!(queue.identity(), Some("entry-1"));
        assert_eq!(queue.run_id(), None);
    }

    fn query_snapshot() -> SessionSnapshot {
        let mut snapshot = SessionSnapshot::default();
        for (record_type, id, seq, data) in [
            (
                "operation_started",
                "run-1",
                1,
                serde_json::json!({"id":"run-1","intent":{"kind":"run"}}),
            ),
            (
                "step_attempt",
                "run-1",
                2,
                serde_json::json!({"runId":"run-1","step":"assistant","attempt":1,"resultEntryId":"entry-1"}),
            ),
            (
                "operation_started",
                "run-2",
                3,
                serde_json::json!({"id":"run-2","intent":{"kind":"compaction"}}),
            ),
        ] {
            snapshot.lane_records.push(SessionLaneRecordSnapshot {
                record_type: record_type.into(),
                id: id.into(),
                lane: Some("main".into()),
                seq: Some(seq),
                timestamp: Some(seq as i64),
                data,
            });
        }

        snapshot
    }

    #[test]
    fn lane_query_filters_pi_records_without_reordering_the_snapshot() {
        let snapshot = query_snapshot();
        let records = snapshot.find_lane_records(&SessionLaneQuery {
            run_id: Some("run-1".into()),
            after_seq: Some(1),
            newest_first: true,
            limit: Some(1),
            ..SessionLaneQuery::default()
        });
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].record_type, "step_attempt");
        assert_eq!(records[0].seq, Some(2));

        let compactions = snapshot.find_lane_records(&SessionLaneQuery {
            record_type: Some("operation_started".into()),
            operation_kind: Some("compaction".into()),
            ..SessionLaneQuery::default()
        });
        assert_eq!(compactions[0].id, "run-2");
    }
