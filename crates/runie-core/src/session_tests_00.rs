    use super::*;
    use crate::events::EventBus;
    use crate::types::{
        AssistantContent, AssistantMessage, DeferredHandle, ToolCall, ToolResultContent,
        ToolResultMessage, Usage, UserContent, UserMessage,
    };

    pub(super) fn user(text: &str) -> AgentMessage {
        AgentMessage::User(UserMessage {
            content: vec![UserContent::Text { text: text.into() }],
            timestamp: 7,
        })
    }

    async fn label(actor: &SessionActor, target: &str, value: Option<&str>) -> Result<(), String> {
        actor
            .apply_event(&AgentEvent::SessionLabelChanged {
                target_id: target.into(),
                label: value.map(str::to_owned),
            })
            .await
    }

    async fn name(actor: &SessionActor, value: &str) -> Result<(), String> {
        actor
            .apply_event(&AgentEvent::SessionNameChanged { name: value.into() })
            .await
    }

    async fn lane(
        actor: &SessionActor,
        name: &str,
        leaf: Option<&str>,
        create: bool,
    ) -> Result<(), String> {
        actor
            .record_lane(name.into(), leaf.map(str::to_owned), create)
            .await
    }

    fn assert_lane_projection(snapshot: &SessionSnapshot) -> SessionSnapshot {
        assert_eq!(snapshot.entries.len(), 2);
        assert_eq!(snapshot.entry_lane("entry-2"), Some("feature"));
        assert_eq!(snapshot.entries[1].parent_id.as_deref(), Some("entry-1"));
        assert_eq!(snapshot.branch_entry_ids_for_lane("feature"), vec!["entry-1", "entry-2"]);
        assert_eq!(snapshot.entries_for_lane("feature").iter().map(|entry| entry.id.as_str()).collect::<Vec<_>>(), vec!["entry-1", "entry-2"]);
        assert_eq!(snapshot.find_entries(&SessionEntryQuery { lane: Some("feature".into()), ..SessionEntryQuery::default() }).into_iter().filter_map(|entry| match entry { SessionEntryRecord::Message(entry) => Some(entry.id), SessionEntryRecord::Config(_) => None }).collect::<Vec<_>>(), vec!["entry-2"]);
        assert_eq!(snapshot.lanes().get("feature"), Some(&Some("entry-2".into())));
        let mut canonical = snapshot.clone();
        canonical.entry_lanes.clear();
        canonical
    }

    #[tokio::test]
    async fn actor_reduces_ordered_entries_and_parent_links() {
        let actor = SessionActor::new();
        actor.append(user("one")).await;
        actor.append(user("two")).await;
        let snapshot = actor.snapshot();
        assert_eq!(snapshot.sequence, 2);
        assert_eq!(snapshot.entries[0].parent_id, None);
        assert_eq!(snapshot.entries[1].parent_id.as_deref(), Some("entry-1"));
        assert_eq!(snapshot.leaf_id.as_deref(), Some("entry-2"));
    }

    #[tokio::test]
    async fn fork_command_replaces_actor_snapshot_at_validated_target() {
        let actor = SessionActor::new();
        actor.append(user("one")).await;
        actor.append(user("two")).await;
        actor
            .fork_at_message("entry-1".into())
            .await
            .expect("fork through actor mailbox");
        let snapshot = actor.snapshot();
        assert_eq!(snapshot.branch_entry_ids(), vec!["entry-1"]);
        assert_eq!(snapshot.entries.len(), 1);
        assert!(actor.fork_at_message("missing".into()).await.is_err());
        assert_eq!(actor.snapshot().entries.len(), 1);
    }

    #[tokio::test]
    async fn restore_snapshot_replaces_state_only_through_import_mailbox() {
        let source = SessionActor::new();
        source.append(user("restored")).await;
        let target = SessionActor::new();
        target.append(user("discarded")).await;
        target
            .restore_snapshot(source.snapshot())
            .await
            .expect("restore snapshot");
        assert_eq!(target.snapshot().entries[0].message, user("restored"));
    }

    #[tokio::test]
    async fn tree_command_moves_leaf_without_discarding_alternate_entries() {
        let actor = SessionActor::new();
        actor.append(user("one")).await;
        actor.append(user("two")).await;
        actor
            .select_tree("entry-1".into())
            .await
            .expect("tree select");
        let snapshot = actor.snapshot();
        assert_eq!(snapshot.entries.len(), 2);
        assert_eq!(snapshot.leaf_id.as_deref(), Some("entry-1"));
        assert_eq!(snapshot.branch_entry_ids(), vec!["entry-1"]);
        assert!(actor.select_tree("missing".into()).await.is_err());
        assert_eq!(actor.snapshot().leaf_id.as_deref(), Some("entry-1"));
    }

    #[tokio::test]
    async fn navigation_admission_validates_targets_before_journaling() {
        let actor = SessionActor::new();
        actor.append(user("target")).await;
        actor
            .admit_navigation(
                "navigation-1".into(),
                "main".into(),
                "entry-1".into(),
                true,
                None,
            )
            .await
            .expect("valid navigation");
        assert!(actor
            .admit_navigation(
                "navigation-2".into(),
                "main".into(),
                "missing".into(),
                false,
                None,
            )
            .await
            .is_err());
        assert_eq!(actor.snapshot().active_operations.len(), 1);
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn labels_are_event_reduced_validated_and_removed_by_fact() {
        let actor = SessionActor::new();
        actor.append(user("one")).await;
        label(&actor, "entry-1", Some("important"))
            .await
            .expect("label admission");
        assert_eq!(
            actor.snapshot().labels().get("entry-1"),
            Some(&"important".to_owned())
        );

        label(&actor, "entry-1", None).await.expect("label removal");
        assert!(actor.snapshot().labels().is_empty());
        name(&actor, "demo").await.expect("name admission");
        assert_eq!(actor.snapshot().name().as_deref(), Some("demo"));
        let error = label(&actor, "missing", Some("bad"))
            .await
            .expect_err("missing target must be rejected");
        assert!(error.contains("label target does not exist"));
        label(&actor, "entry-1", Some("important"))
            .await
            .expect("label before fork");
        let fork = actor
            .snapshot()
            .fork_at_message("entry-1")
            .expect("fork facts");
        assert_eq!(fork.name().as_deref(), Some("demo"));
        assert_eq!(fork.labels().get("entry-1"), Some(&"important".to_owned()));
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn lane_events_are_validated_projected_and_jsonl_round_tripped() {
        let actor = SessionActor::new();
        actor.append(user("one")).await;
        lane(&actor, "feature", Some("entry-1"), true)
            .await
            .expect("lane admission");
        actor
            .record_config(SessionConfigRecord::NameChanged {
                name: "after-lane".into(),
            })
            .await
            .expect("config after lane");
        assert_eq!(
            actor.snapshot().lanes().get("feature"),
            Some(&Some("entry-1".into()))
        );
        let jsonl = actor.snapshot().to_jsonl("s", 1, "/tmp");
        let serialized: Vec<u64> = jsonl
            .lines()
            .skip(1)
            .map(|line| {
                serde_json::from_str::<serde_json::Value>(line).unwrap()["seq"]
                    .as_u64()
                    .unwrap()
            })
            .collect();
        assert_eq!(serialized, vec![1, 2, 3]);
        let (_, _, imported) = SessionSnapshot::from_jsonl(&jsonl).expect("lane JSONL");
        assert_eq!(imported.lanes(), actor.snapshot().lanes());
        assert!(lane(&actor, "feature", Some("entry-1"), true).await.is_err());
        assert!(lane(&actor, "missing-lane", None, false).await.is_err());
        assert!(lane(&actor, "feature", Some("missing"), false).await.is_err());
        assert_eq!(actor.snapshot().lane_facts.len(), 1);
    }

    #[tokio::test]
    #[allow(
        clippy::too_many_lines,
        reason = "the lane append regression covers persistence and all projections"
    )]
    async fn append_to_lane_updates_only_that_lane_and_persists_identity() {
        let actor = SessionActor::new();
        actor.append(user("main")).await;
        lane(&actor, "feature", Some("entry-1"), true)
            .await
            .expect("lane create");
        actor
            .append_to_lane("feature".into(), user("feature"))
            .await
            .expect("lane append");
        let snapshot = actor.snapshot();
        let canonical_snapshot = assert_lane_projection(&snapshot);
        let lane_fork = canonical_snapshot
            .fork_at_lane_message("feature", "entry-2")
            .expect("feature lane fork");
        assert_eq!(lane_fork.entries.len(), 2);
        assert_eq!(lane_fork.entry_lane("entry-2"), Some("feature"));
        let (_, _, imported) =
            SessionSnapshot::from_jsonl(&canonical_snapshot.to_jsonl("s", 1, "/tmp"))
                .expect("lane append JSONL");
        assert_eq!(imported.entry_lane("entry-2"), Some("feature"));
    }

    #[tokio::test]
    async fn lane_can_move_to_a_configuration_entry_in_the_shared_tree() {
        let actor = SessionActor::new();
        actor.append(user("root")).await;
        actor
            .record_config(SessionConfigRecord::BranchSummaryCreated {
                from_id: "entry-1".into(),
                summary: "branch summary".into(),
                details: None,
            })
            .await
            .expect("summary entry");
        lane(&actor, "feature", Some("entry-1"), true)
            .await
            .expect("lane create");
        lane(&actor, "feature", Some("entry-2"), false)
            .await
            .expect("move to shared config entry");
        assert_eq!(
            actor.snapshot().lanes().get("feature"),
            Some(&Some("entry-2".into()))
        );
    }

    #[test]
    fn find_entries_filters_configuration_records_by_their_lane() {
        let snapshot = SessionSnapshot {
            config_records: vec![SessionConfigEntry {
                id: "config-1".into(),
                lane: "feature".into(),
                seq: 1,
                parent_id: None,
                timestamp: 0,
                record: SessionConfigRecord::NameChanged {
                    name: "feature".into(),
                },
            }],
            ..SessionSnapshot::default()
        };
        let feature = snapshot.find_entries(&SessionEntryQuery {
            lane: Some("feature".into()),
            ..SessionEntryQuery::default()
        });
        assert_eq!(feature.len(), 1);
        assert_eq!(snapshot.entry_lane("config-1"), Some("feature"));
        assert!(snapshot
            .find_entries(&SessionEntryQuery {
                lane: Some("main".into()),
                ..SessionEntryQuery::default()
            })
            .is_empty());
    }

    #[tokio::test]
    async fn append_custom_entry_is_actor_owned_and_round_trips() {
        let actor = SessionActor::new();
        actor.append(user("before")).await;
        actor
            .append_custom_entry(
                "replay.marker".into(),
                Some(serde_json::json!({"source": "yaml"})),
            )
            .await
            .expect("custom entry");

        let snapshot = actor.snapshot();
        let custom = snapshot.find_entries(&SessionEntryQuery {
            record_type: Some("custom".into()),
            custom_type: Some("replay.marker".into()),
            ..SessionEntryQuery::default()
        });
        assert_eq!(custom.len(), 1);
        let jsonl = snapshot.to_jsonl("session", 1, "/tmp");
        let (_, _, restored) = SessionSnapshot::from_jsonl(&jsonl).expect("custom JSONL");
        assert_eq!(
            restored.find_entries(&SessionEntryQuery {
                record_type: Some("custom".into()),
                custom_type: Some("replay.marker".into()),
                ..SessionEntryQuery::default()
            }),
            custom
        );
        assert!(actor.append_custom_entry(" ".into(), None).await.is_err());
    }

    #[tokio::test]
    async fn bus_message_end_and_reset_are_the_only_projection_inputs() {
        let bus = EventBus::new();
        let actor = SessionActor::new_with_bus(&bus);
        let _ = actor
            .record_operation(
                SessionOperationKind::Started,
                serde_json::json!({"id": "run-1"}),
            )
            .await;
        let _ = actor
            .record_operation(
                SessionOperationKind::Started,
                serde_json::json!({"id": "run-1"}),
            )
            .await;
        bus.publish(AgentEvent::MessageEnd {
            message: user("one"),
        });
        tokio::task::yield_now().await;
        assert_eq!(actor.snapshot().entries.len(), 1);
        bus.publish(AgentEvent::Reset);
        tokio::task::yield_now().await;
        assert!(actor.snapshot().entries.is_empty());
    }
