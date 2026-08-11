    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the context usage vector keeps all Pi edge cases together"
    )]
    fn context_usage_prefers_latest_valid_assistant_usage_and_estimates_tail() {
        const EIGHT_TOKENS_OF_TEXT: &str = "12345678";
        const ONE_TOKEN_OF_TEXT: &str = "1234";
        let messages = vec![
            user(EIGHT_TOKENS_OF_TEXT),
            AgentMessage::Assistant(crate::types::AssistantMessage {
                usage: crate::types::Usage {
                    input: 10,
                    output: 5,
                    total_tokens: 0,
                    ..Default::default()
                },
                stop_reason: Some(StopReason::Stop),
                ..Default::default()
            }),
            user(ONE_TOKEN_OF_TEXT),
        ];
        assert_eq!(estimate_message_tokens(&messages[0]), 2);
        assert_eq!(estimate_message_tokens(&user("😀😀😀")), 2);
        assert_eq!(
            estimate_context_tokens(&messages),
            ContextUsageEstimate {
                tokens: 16,
                usage_tokens: 15,
                trailing_tokens: 1,
                last_usage_index: Some(1),
            }
        );
        assert_eq!(
            aborted_context_usage(),
            ContextUsageEstimate {
                tokens: 0,
                usage_tokens: 0,
                trailing_tokens: 0,
                last_usage_index: None,
            }
        );
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        clippy::cognitive_complexity,
        reason = "the regression covers request construction and stale-plan rejection"
    )]
    fn compaction_preparation_partitions_history_prefix_and_tail() {
        let entries = compaction_entries();
        let preparation = prepare_compaction_entries(&entries, &[40, 40], 20)
            .expect("preparation")
            .expect("non-empty");
        assert!(preparation.history_indices.is_empty());
        assert_eq!(preparation.turn_prefix_indices, vec![0]);
        assert_eq!(preparation.retained_indices, vec![1]);
        assert_eq!(preparation.tokens_before, 80);
        let request = CompactionSummaryRequest::from_preparation(&preparation, &entries, None)
            .expect("provider request");
        assert_eq!(request.history, Vec::<AgentMessage>::new());
        assert_eq!(request.turn_prefix, vec![user("request")]);
        assert!(matches!(
            request.retained_tail.as_slice(),
            [AgentMessage::Assistant(_)]
        ));
        let invalid = CompactionPreparation {
            history_indices: vec![99],
            ..preparation.clone()
        };
        assert!(CompactionSummaryRequest::from_preparation(&invalid, &entries, None).is_err());
        let mut stale = preparation.clone();
        stale.source_entry_ids[0] = "different-entry".into();
        assert!(CompactionSummaryRequest::from_preparation(&stale, &entries, None).is_err());
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the regression covers both event construction and stale-index rejection"
    )]
    fn provider_compaction_summary_builds_validated_publication_event() {
        let entries = compaction_entries();
        let preparation = prepare_compaction_entries(&entries, &[40, 40], 20)
            .expect("preparation")
            .expect("non-empty");
        let event = CompactionSummary {
            summary: "prior context summarized".into(),
            usage: None,
            details: Some(serde_json::json!({"source":"provider"})),
        }
        .into_event(&preparation, &entries)
        .expect("publication event");

        assert!(matches!(
            event,
            AgentEvent::CompactionCreated {
                ref summary,
                ref retained_tail,
                tokens_before: 80,
                ..
            } if summary == "prior context summarized" && retained_tail.len() == 1
        ));

        let invalid = CompactionPreparation {
            retained_indices: vec![99],
            ..preparation
        };
        assert!(CompactionSummary {
            summary: "invalid".into(),
            usage: None,
            details: None,
        }
        .into_event(&invalid, &entries)
        .is_err());
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the regression fixture spells out the parent-linked journal boundary"
    )]
    fn compaction_context_projection_drops_prefix_and_deferred_results() {
        let snapshot = SessionSnapshot {
            sequence: 4,
            leaf_id: Some("entry-4".into()),
            entries: vec![
                SessionEntry {
                    id: "entry-1".into(),
                    lane: "main".into(),
                    seq: 1,
                    parent_id: None,
                    timestamp: 0,
                    message: user("old"),
                    terminate: false,
                },
                SessionEntry {
                    id: "entry-2".into(),
                    lane: "main".into(),
                    seq: 2,
                    parent_id: Some("entry-1".into()),
                    timestamp: 0,
                    message: AgentMessage::Assistant(Default::default()),
                    terminate: false,
                },
                SessionEntry {
                    id: "entry-4".into(),
                    lane: "main".into(),
                    seq: 4,
                    parent_id: Some("entry-3".into()),
                    timestamp: 0,
                    message: AgentMessage::Assistant(AssistantMessage {
                        stop_reason: Some(StopReason::Deferred),
                        ..Default::default()
                    }),
                    terminate: false,
                },
            ],
            config_records: vec![SessionConfigEntry {
                id: "entry-3".into(),
                lane: "main".into(),
                seq: 3,
                parent_id: Some("entry-2".into()),
                timestamp: 0,
                record: SessionConfigRecord::CompactionCreated {
                    summary: "summary".into(),
                    retained_tail: vec![user("retained")],
                    tokens_before: 80,
                    details: None,
                    usage: None,
                },
            }],
            ..Default::default()
        };
        let projection = snapshot
            .compaction_context_projection()
            .expect("latest compaction");
        assert_eq!(projection.summary, "summary");
        assert_eq!(projection.tokens_before, 80);
        assert_eq!(projection.retained_tail.len(), 1);
        assert!(projection.message_indices.is_empty());
        assert_eq!(
            projection.terminal_lines(),
            vec![
                "Compaction summary: summary",
                "Tokens before: 80",
                "Retained messages: 1",
                "Context message indices: []",
            ]
        );
    }

    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the regression spells out a divergent parent-linked tree"
    )]
    fn branch_context_materializes_selected_path_and_compaction_boundary() {
        let snapshot = SessionSnapshot {
            leaf_id: Some("branch-tail".into()),
            entries: vec![
                SessionEntry {
                    id: "root".into(),
                    lane: "main".into(),
                    seq: 1,
                    parent_id: None,
                    timestamp: 0,
                    message: user("root"),
                    terminate: false,
                },
                SessionEntry {
                    id: "other-branch".into(),
                    lane: "main".into(),
                    seq: 2,
                    parent_id: Some("root".into()),
                    timestamp: 0,
                    message: user("not selected"),
                    terminate: false,
                },
                SessionEntry {
                    id: "branch-tail".into(),
                    lane: "main".into(),
                    seq: 5,
                    parent_id: Some("summary".into()),
                    timestamp: 0,
                    message: user("selected tail"),
                    terminate: false,
                },
            ],
            config_records: vec![SessionConfigEntry {
                id: "summary".into(),
                lane: "main".into(),
                seq: 4,
                parent_id: Some("root".into()),
                timestamp: 9,
                record: SessionConfigRecord::CompactionCreated {
                    summary: "earlier context".into(),
                    retained_tail: vec![user("retained")],
                    tokens_before: 42,
                    details: None,
                    usage: None,
                },
            }],
            ..Default::default()
        };

        assert_eq!(
            snapshot.branch_context_messages(),
            vec![
                AgentMessage::CompactionSummary(crate::types::CompactionSummaryMessage {
                    summary: "earlier context".into(),
                    tokens_before: 42,
                    timestamp: 9,
                }),
                user("retained"),
                user("selected tail"),
            ]
        );
    }
    #[tokio::test]
    #[allow(
        clippy::too_many_lines,
        reason = "the actor regression covers fresh and stale compaction plans"
    )]
    async fn actor_owns_compaction_preparation_through_mailbox() {
        let actor = SessionActor::new();
        actor.append(user("request")).await;
        actor
            .append(AgentMessage::Assistant(Default::default()))
            .await;
        let preparation = actor
            .prepare_compaction(vec![40, 40], 20)
            .await
            .expect("actor response")
            .expect("non-empty preparation");
        assert_eq!(preparation.retained_indices, vec![1]);
        assert_eq!(actor.snapshot().entries.len(), 2);
        let stale = preparation.clone();
        actor.append(user("changed journal")).await;
        assert!(actor
            .publish_compaction(
                stale,
                CompactionSummary {
                    summary: "stale".into(),
                    usage: None,
                    details: None,
                },
            )
            .await
            .is_err());
        let preparation = actor
            .prepare_compaction(vec![40, 40, 40], 20)
            .await
            .expect("fresh actor response")
            .expect("fresh non-empty preparation");
        actor
            .publish_compaction(
                preparation,
                CompactionSummary {
                    summary: "provider summary".into(),
                    usage: None,
                    details: None,
                },
            )
            .await
            .expect("compaction publication");
        let snapshot = actor.snapshot();
        assert_eq!(snapshot.config_records.len(), 1);
        assert!(matches!(
            snapshot.config_records[0].record,
            SessionConfigRecord::CompactionCreated {
                ref summary,
                ref retained_tail,
                tokens_before: 120,
                ..
            } if summary == "provider summary" && retained_tail.len() == 1
        ));
    }

    #[test]
    fn compaction_preparation_round_trips_as_replay_data() {
        let preparation = prepare_compaction_entries(&compaction_entries(), &[40, 40], 20)
            .unwrap()
            .unwrap();
        let encoded = serde_json::to_string(&preparation).unwrap();
        let decoded: CompactionPreparation = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, preparation);
    }

fn compaction_entries() -> Vec<SessionEntry> {
    vec![
        SessionEntry { id: "user-1".into(), lane: "main".into(), seq: 1, parent_id: None, timestamp: 0, message: user("request"), terminate: false },
        SessionEntry { id: "assistant-1".into(), lane: "main".into(), seq: 2, parent_id: Some("user-1".into()), timestamp: 0, message: AgentMessage::Assistant(Default::default()), terminate: false },
    ]
}

fn aborted_context_usage() -> ContextUsageEstimate {
    estimate_context_tokens(&[AgentMessage::Assistant(crate::types::AssistantMessage {
        usage: crate::types::Usage { total_tokens: 100, ..Default::default() },
        stop_reason: Some(StopReason::Aborted),
        ..Default::default()
    })])
}
