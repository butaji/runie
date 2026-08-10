impl SessionSnapshot {
    /// Return the selected branch from oldest to newest journal node.
    /// Message and configuration records share the same parent/id namespace.
    pub fn branch_entry_ids(&self) -> Vec<String> {
        self.branch_entry_ids_from_leaf(self.leaf_id.clone())
    }

    /// Return the selected branch for one Pi session lane.
    pub fn branch_entry_ids_for_lane(&self, lane: &str) -> Vec<String> {
        self.branch_entry_ids_from_leaf(self.lanes().get(lane).cloned().flatten())
    }

    /// Return message entries on one lane's selected branch, oldest first.
    pub fn entries_for_lane(&self, lane: &str) -> Vec<SessionEntry> {
        let ids = self
            .branch_entry_ids_for_lane(lane)
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        self.entries
            .iter()
            .filter(|entry| ids.contains(&entry.id))
            .cloned()
            .collect()
    }

    /// Materialize provider context for the selected parent-linked branch.
    ///
    /// Pi builds context from the selected leaf path, not from every message
    /// in the session file.  A compaction on that path replaces its earlier
    /// prefix with the summary and retained tail; deferred assistant results
    /// are journal facts but are not sent to the provider.
    pub fn branch_context_messages(&self) -> Vec<AgentMessage> {
        self.branch_context_messages_for_leaf(self.leaf_id.clone())
    }

    /// Materialize provider context for a named lane's selected leaf.
    pub fn branch_context_messages_for_lane(&self, lane: &str) -> Vec<AgentMessage> {
        self.branch_context_messages_for_leaf(self.lanes().get(lane).cloned().flatten())
    }

    fn branch_context_messages_for_leaf(&self, leaf_id: Option<String>) -> Vec<AgentMessage> {
        let branch = self
            .branch_entry_ids_from_leaf(leaf_id)
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let messages = self.branch_context_entries(&branch);
        let compaction = self.branch_compaction(&branch);
        let Some((compaction_seq, timestamp, summary, retained_tail, tokens_before)) = compaction
        else {
            return messages.into_iter().map(|(_, message)| message).collect();
        };
        project_compacted_context(
            messages,
            compaction_seq,
            timestamp,
            summary,
            retained_tail,
            tokens_before,
        )
    }

    fn branch_context_entries(
        &self,
        branch: &std::collections::BTreeSet<String>,
    ) -> Vec<(u64, AgentMessage)> {
        let mut messages = self
            .entries
            .iter()
            .filter(|entry| branch.contains(&entry.id))
            .filter(|entry| is_provider_context_message(&entry.message))
            .map(|entry| (entry.seq, entry.message.clone()))
            .collect::<Vec<_>>();
        messages.sort_by_key(|(seq, _)| *seq);
        messages
    }

    fn branch_compaction(
        &self,
        branch: &std::collections::BTreeSet<String>,
    ) -> Option<(u64, i64, String, Vec<AgentMessage>, u64)> {
        self.config_records
            .iter()
            .filter(|entry| branch.contains(&entry.id))
            .filter_map(|entry| match &entry.record {
                SessionConfigRecord::CompactionCreated {
                    summary,
                    retained_tail,
                    tokens_before,
                    ..
                } => Some((
                    entry.seq,
                    entry.timestamp,
                    summary.clone(),
                    retained_tail.clone(),
                    *tokens_before,
                )),
                _ => None,
            })
            .max_by_key(|(seq, ..)| *seq)
    }

    fn branch_entry_ids_from_leaf(&self, leaf_id: Option<String>) -> Vec<String> {
        let mut parents = BTreeMap::new();
        for entry in &self.entries {
            parents.insert(entry.id.clone(), entry.parent_id.clone());
        }
        for entry in &self.config_records {
            parents.insert(entry.id.clone(), entry.parent_id.clone());
        }
        let mut path = Vec::new();
        let mut current = leaf_id;
        while let Some(id) = current {
            if path.iter().any(|seen| seen == &id) {
                break;
            }
            current = parents.get(&id).cloned().flatten();
            path.push(id);
        }
        path.reverse();
        path
    }

    /// Create the message-lane fork prefix Pi would publish into a new
    /// session. The returned snapshot owns new sequence numbers while
    /// retaining the original parent/id graph; no actor or source snapshot
    /// is mutated.
    #[allow(
        clippy::too_many_lines,
        reason = "fork validation and projection stay one pure operation"
    )]
    pub fn fork_at_message(&self, target_id: &str) -> Result<Self, String> {
        self.fork_from_branch(target_id, self.branch_entry_ids())
    }

    /// Fork a validated message target from a named Pi session lane.
    pub fn fork_at_lane_message(&self, lane: &str, target_id: &str) -> Result<Self, String> {
        self.fork_from_branch(target_id, self.branch_entry_ids_for_lane(lane))
    }

    /// Return the parent node for an actor-owned undo navigation. Undo only
    /// changes the selected leaf; journal entries and alternate branches stay
    /// intact for replay and redo-like navigation.
    pub fn undo_target(&self) -> Result<String, String> {
        let current = self
            .leaf_id
            .as_deref()
            .ok_or_else(|| "session has no selected leaf".to_owned())?;
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.id == current)
            .ok_or_else(|| format!("selected leaf does not exist: {current}"))?;
        entry
            .parent_id
            .clone()
            .ok_or_else(|| "session is already at the root".to_owned())
    }

    fn fork_from_branch(&self, target_id: &str, branch: Vec<String>) -> Result<Self, String> {
        self.validate_fork_target(target_id, &branch)?;
        let retained = branch
            .into_iter()
            .take_while(|id| id != target_id)
            .chain(std::iter::once(target_id.to_owned()))
            .collect::<std::collections::BTreeSet<_>>();
        let mut fork = Self {
            leaf_id: Some(target_id.to_owned()),
            ..Self::default()
        };
        let mut sequence = self.copy_fork_entries(&retained, &mut fork);
        sequence = self.copy_fork_config(&retained, &mut fork, sequence);
        sequence = self.append_fork_facts(target_id, &retained, &mut fork, sequence);
        fork.sequence = sequence;
        Ok(fork)
    }

    fn validate_fork_target(&self, target_id: &str, branch: &[String]) -> Result<(), String> {
        if !self.entries.iter().any(|entry| entry.id == target_id) {
            return Err(format!("invalid fork target {target_id:?}"));
        }
        branch
            .iter()
            .any(|id| id == target_id)
            .then_some(())
            .ok_or_else(|| format!("fork target {target_id:?} is not on the selected branch"))
    }

    fn copy_fork_entries(
        &self,
        retained: &std::collections::BTreeSet<String>,
        fork: &mut Self,
    ) -> u64 {
        let mut sequence = 0;
        for entry in &self.entries {
            if !retained.contains(&entry.id) {
                continue;
            }
            sequence += 1;
            let mut copy = entry.clone();
            copy.seq = sequence;
            fork.entry_lanes.insert(copy.id.clone(), copy.lane.clone());
            fork.entries.push(copy);
        }
        sequence
    }

    fn copy_fork_config(
        &self,
        retained: &std::collections::BTreeSet<String>,
        fork: &mut Self,
        mut sequence: u64,
    ) -> u64 {
        for entry in &self.config_records {
            if !retained.contains(&entry.id)
                || matches!(
                    entry.record,
                    SessionConfigRecord::NameChanged { .. }
                        | SessionConfigRecord::LabelChanged { .. }
                )
            {
                continue;
            }
            sequence += 1;
            let mut copy = entry.clone();
            copy.seq = sequence;
            if let Some((record_type, data)) = operation_record_parts(&copy.record) {
                reduce_operation_record(fork, record_type, data);
            }
            fork.config_records.push(copy);
        }
        sequence
    }

    fn append_fork_facts(
        &self,
        target_id: &str,
        retained: &std::collections::BTreeSet<String>,
        fork: &mut Self,
        mut sequence: u64,
    ) -> u64 {
        // Pi forks publish a fresh main-lane pointer after the copied entry
        // prefix. Lane facts from the source are not copied verbatim; the
        // fork receives one authoritative pointer for its new tree.
        sequence += 1;
        fork.lane_facts.push(SessionLaneFact {
            seq: sequence,
            lane: "main".into(),
            leaf_id: Some(target_id.to_owned()),
        });
        sequence = self.append_fork_name(target_id, fork, sequence);
        self.append_fork_labels(target_id, retained, fork, sequence)
    }

    fn append_fork_name(&self, target_id: &str, fork: &mut Self, mut sequence: u64) -> u64 {
        let Some(name) = self.name() else {
            return sequence;
        };
        sequence += 1;
        fork.config_records.push(SessionConfigEntry {
            id: format!("fork-fact-{sequence}"),
            lane: "main".into(),
            seq: sequence,
            parent_id: Some(target_id.to_owned()),
            timestamp: 0,
            record: SessionConfigRecord::NameChanged { name },
        });
        sequence
    }

    fn append_fork_labels(
        &self,
        target_id: &str,
        retained: &std::collections::BTreeSet<String>,
        fork: &mut Self,
        mut sequence: u64,
    ) -> u64 {
        for (label_target, label) in self
            .labels()
            .into_iter()
            .filter(|(id, _)| retained.contains(id))
        {
            sequence += 1;
            fork.config_records.push(SessionConfigEntry {
                id: format!("fork-fact-{sequence}"),
                lane: "main".into(),
                seq: sequence,
                parent_id: Some(target_id.to_owned()),
                timestamp: 0,
                record: SessionConfigRecord::LabelChanged {
                    target_id: label_target,
                    label: Some(label),
                },
            });
        }
        sequence
    }

    /// Validate the currently projected navigation intent against journal IDs.
    /// This is pure and intentionally does not admit or mutate navigation.
    pub fn navigation_validation(&self) -> Option<NavigationValidation> {
        let ids = self
            .entries
            .iter()
            .map(|entry| entry.id.as_str())
            .chain(self.config_records.iter().map(|entry| entry.id.as_str()))
            .collect::<std::collections::BTreeSet<_>>();
        self.navigation
            .as_ref()
            .map(|navigation| NavigationValidation {
                target_exists: navigation
                    .target_id
                    .as_deref()
                    .is_some_and(|target| ids.contains(target)),
                summary_exists: navigation
                    .summary_entry_id
                    .as_deref()
                    .is_some_and(|summary| ids.contains(summary)),
            })
    }

    /// Parse the message-only subset emitted by [`Self::to_jsonl`].
    /// Validation follows Pi's v4 invariants for header, sequence, and parent
    /// linkage; unsupported mutation kinds are rejected explicitly.
    ///
    /// The filesystem actor should call [`Self::repair_jsonl_torn_tail`] before
    /// handing file contents to this parser. Keeping repair pure makes the
    /// recovery decision deterministic and leaves publication to the storage
    /// actor.
    pub fn from_jsonl(input: &str) -> Result<(String, String, Self), String> {
        let (session_id, cwd, lines) = parse_jsonl_header(input)?;
        let mut snapshot = Self::default();
        for (line_index, line) in lines.into_iter().enumerate() {
            parse_jsonl_line(&mut snapshot, line, line_index + 2)?;
        }
        Ok((session_id, cwd, snapshot))
    }

    /// Repair the one failure Pi's JSONL loader may recover locally: an
    /// unterminated or invalid final physical line. A malformed non-final
    /// line is never discarded. Valid final content is only normalized by
    /// appending its missing newline; no mutation is interpreted here.
    pub fn repair_jsonl_torn_tail(input: &str) -> Result<String, String> {
        let mut lines = input.split('\n').collect::<Vec<_>>();
        while lines.last().is_some_and(|line| line.is_empty()) {
            lines.pop();
        }
        let header = lines
            .first()
            .ok_or_else(|| "session JSONL is empty".to_owned())?;
        serde_json::from_str::<serde_json::Value>(header)
            .map_err(|error| format!("invalid session header: {error}"))?;
        for (index, line) in lines.iter().enumerate().skip(1) {
            if serde_json::from_str::<serde_json::Value>(line).is_err() {
                if index + 1 != lines.len() {
                    return Err(format!("invalid session entry {}", index + 1));
                }
                lines.truncate(index);
                return Ok(format!("{}\n", lines.join("\n")));
            }
        }
        Ok(format!("{}\n", lines.join("\n")))
    }

    /// Encode the message lane using Pi's JSONL v4 header/entry shape.
    /// Filesystem writes stay outside this pure projection function.
    fn message_jsonl_entries(&self) -> Vec<String> {
        self.entries
            .iter()
            .map(|session_entry| {
                let mut entry = serde_json::json!({
                    "kind": "entry",
                    "lane": session_entry.lane.as_str(),
                    "type": "message",
                    "id": session_entry.id,
                    "parentId": session_entry.parent_id,
                    "seq": session_entry.seq,
                    "timestamp": session_entry.timestamp,
                    "message": session_entry.message,
                });
                if session_entry.terminate {
                    entry["terminate"] = serde_json::Value::Bool(true);
                }
                entry.to_string()
            })
            .collect()
    }

    pub fn to_jsonl(&self, session_id: &str, created_at: i64, cwd: &str) -> String {
        let mut lines = Vec::with_capacity(self.entries.len() + 1);
        lines.push(
            serde_json::json!({
                "kind": "header",
                "version": 4,
                "id": session_id,
                "createdAt": created_at,
                "cwd": cwd,
            })
            .to_string(),
        );
        let mut entry_lines = self.message_jsonl_entries();
        entry_lines.extend(self.config_jsonl_entries());
        entry_lines.extend(self.lane_fact_jsonl_entries());
        entry_lines.extend(self.lane_record_jsonl_entries());
        entry_lines.sort_by_key(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .ok()
                .and_then(|value| value.get("seq").and_then(serde_json::Value::as_u64))
                .unwrap_or_default()
        });
        lines.extend(entry_lines);
        format!("{}\n", lines.join("\n"))
    }

    fn lane_fact_jsonl_entries(&self) -> Vec<String> {
        self.lane_facts
            .iter()
            .map(|fact| {
                serde_json::json!({
                    "kind": "lane",
                    "lane": fact.lane,
                    "seq": fact.seq,
                    "leafId": fact.leaf_id,
                })
                .to_string()
            })
            .collect()
    }

    fn config_jsonl_entries(&self) -> Vec<String> {
        self.config_records
            .iter()
            .map(|session_entry| {
                let (entry_type, mut entry) = config_record_json(&session_entry.record);
                if matches!(
                    entry_type,
                    "operation_started" | "operation_finished" | "abort_requested"
                ) {
                    if let Some(operation_id) =
                        entry.get("id").cloned().filter(|value| value.is_string())
                    {
                        entry
                            .as_object_mut()
                            .expect("operation record is an object")
                            .entry("runId")
                            .or_insert(operation_id);
                    }
                }
                entry["kind"] = serde_json::json!("entry");
                entry["lane"] = serde_json::json!(session_entry.lane);
                entry["type"] = serde_json::json!(entry_type);
                entry["id"] = serde_json::json!(session_entry.id);
                entry["parentId"] = session_entry
                    .parent_id
                    .clone()
                    .map_or(serde_json::Value::Null, serde_json::Value::String);
                entry["seq"] = serde_json::json!(session_entry.seq);
                entry["timestamp"] = serde_json::json!(session_entry.timestamp);
                entry.to_string()
            })
            .collect()
    }

    fn lane_record_jsonl_entries(&self) -> Vec<String> {
        self.lane_records
            .iter()
            .enumerate()
            .map(|(index, record)| {
                let mut entry = record.data.clone();
                let object = entry
                    .as_object_mut()
                    .expect("session lane record payload must be an object");
                object.insert("kind".into(), serde_json::json!("entry"));
                object.insert(
                    "lane".into(),
                    serde_json::json!(record.lane.as_deref().unwrap_or("main")),
                );
                object.insert("type".into(), serde_json::json!(record.record_type));
                object.insert("id".into(), serde_json::json!(record.id));
                object.insert(
                    "parentId".into(),
                    index
                        .checked_sub(1)
                        .and_then(|previous| self.lane_records.get(previous))
                        .map(|previous| serde_json::json!(previous.id))
                        .unwrap_or(serde_json::Value::Null),
                );
                object.insert(
                    "seq".into(),
                    serde_json::json!(record.seq.unwrap_or(index as u64 + 1)),
                );
                object.insert(
                    "timestamp".into(),
                    serde_json::json!(record.timestamp.unwrap_or(0)),
                );
                entry.to_string()
            })
            .collect()
    }
}
