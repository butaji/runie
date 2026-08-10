impl SessionSnapshot {
    pub fn entry_lane(&self, entry_id: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.id == entry_id)
            .map(|entry| entry.lane.as_str())
            .or_else(|| {
                self.config_records
                    .iter()
                    .find(|entry| entry.id == entry_id)
                    .map(|entry| entry.lane.as_str())
            })
            .or_else(|| self.entry_lanes.get(entry_id).map(String::as_str))
    }

    /// Reduce ordered Pi lane mutations into the latest leaf per lane.
    pub fn lanes(&self) -> BTreeMap<String, Option<String>> {
        let mut changes = vec![(0_u64, "main".to_owned(), None)];
        for entry in &self.entries {
            changes.push((entry.seq, entry.lane.clone(), Some(entry.id.clone())));
        }
        changes.extend(
            self.lane_facts
                .iter()
                .map(|fact| (fact.seq, fact.lane.clone(), fact.leaf_id.clone())),
        );
        changes.sort_by_key(|(seq, _, _)| *seq);
        let mut lanes = BTreeMap::new();
        for (_, lane, leaf_id) in changes {
            lanes.insert(lane, leaf_id);
        }
        lanes
    }

    /// Reduce ordered Pi session-name facts to the latest name.
    pub fn name(&self) -> Option<String> {
        self.config_records.iter().rev().find_map(|entry| {
            if let SessionConfigRecord::NameChanged { name } = &entry.record {
                Some(name.clone())
            } else {
                None
            }
        })
    }

    /// Reduce ordered Pi label facts into the effective label map. This is a
    /// pure read projection; the session actor remains the only writer.
    pub fn labels(&self) -> BTreeMap<String, String> {
        let mut labels = BTreeMap::new();
        for entry in &self.config_records {
            if let SessionConfigRecord::LabelChanged { target_id, label } = &entry.record {
                if let Some(label) = label {
                    labels.insert(target_id.clone(), label.clone());
                } else {
                    labels.remove(target_id);
                }
            }
        }
        labels
    }

    /// Return the first entry selected by Pi's ordered entry query.
    pub fn find_entry(&self, query: &SessionEntryQuery) -> Option<SessionEntryRecord> {
        self.find_entries(query).into_iter().next()
    }

    /// Return the first entry selected by an explicit branch query.
    pub fn find_entry_on_branch(
        &self,
        query: &SessionBranchEntryQuery,
    ) -> Result<Option<SessionEntryRecord>, String> {
        self.find_entries_on_branch(query)
            .map(|entries| entries.into_iter().next())
    }

    /// Find entries on one validated parent-linked branch.
    pub fn find_entries_on_branch(
        &self,
        query: &SessionBranchEntryQuery,
    ) -> Result<Vec<SessionEntryRecord>, String> {
        let parents = self.branch_parents();
        if !parents.contains_key(&query.start) {
            return Err(format!("branch start {:?} was not found", query.start));
        }
        let ids = self.branch_ids(query, &parents)?;
        let mut entries = self.branch_entries(query, &ids);
        if query.newest_first {
            entries.reverse();
        }
        if let Some(limit) = query.limit {
            entries.truncate(limit);
        }
        Ok(entries)
    }

    fn branch_parents(&self) -> BTreeMap<String, Option<String>> {
        self.entries
            .iter()
            .map(|entry| (entry.id.clone(), entry.parent_id.clone()))
            .chain(
                self.config_records
                    .iter()
                    .map(|entry| (entry.id.clone(), entry.parent_id.clone())),
            )
            .collect()
    }

    fn branch_ids(
        &self,
        query: &SessionBranchEntryQuery,
        parents: &BTreeMap<String, Option<String>>,
    ) -> Result<Vec<String>, String> {
        let mut ids = Vec::new();
        let mut current = Some(query.start.clone());
        while let Some(id) = current {
            if ids.iter().any(|seen| seen == &id) {
                return Err("branch contains a parent cycle".into());
            }
            ids.push(id.clone());
            let entry = self
                .find_entries(&SessionEntryQuery::default())
                .into_iter()
                .find(|entry| match entry {
                    SessionEntryRecord::Message(entry) => entry.id == id,
                    SessionEntryRecord::Config(entry) => entry.id == id,
                });
            let Some(entry) = entry else { break };
            if query.stop_at_id.as_deref() == Some(id.as_str())
                || query.stop_at_type.as_deref() == Some(entry.record_type())
            {
                break;
            }
            current = parents.get(&id).cloned().flatten();
        }
        ids.reverse();
        Ok(ids)
    }

    fn branch_entries(
        &self,
        query: &SessionBranchEntryQuery,
        ids: &[String],
    ) -> Vec<SessionEntryRecord> {
        let id_set = ids.iter().collect::<std::collections::HashSet<_>>();
        let mut entries = self
            .find_entries(&SessionEntryQuery {
                record_type: query.record_type.clone(),
                custom_type: query.custom_type.clone(),
                ..SessionEntryQuery::default()
            })
            .into_iter()
            .filter(|entry| match entry {
                SessionEntryRecord::Message(entry) => {
                    id_set.contains(&entry.id)
                        && query
                            .lane
                            .as_deref()
                            .is_none_or(|lane| self.entry_lane(&entry.id) == Some(lane))
                }
                SessionEntryRecord::Config(entry) => {
                    id_set.contains(&entry.id)
                        && query.lane.as_deref().is_none_or(|lane| entry.lane == lane)
                }
            })
            .collect::<Vec<_>>();
        if query.newest_first {
            entries.reverse();
        }
        if let Some(limit) = query.limit {
            entries.truncate(limit);
        }
        entries
    }

    /// Return message/config entries and operation records in journal order.
    pub fn get_log(&self, after_seq: Option<u64>, limit: Option<usize>) -> Vec<SessionLogItem> {
        let mut items = self
            .find_entries(&SessionEntryQuery {
                after_seq,
                ..SessionEntryQuery::default()
            })
            .into_iter()
            .map(|entry| SessionLogItem::Entry {
                seq: entry.seq(),
                entry,
            })
            .chain(self.lane_records.iter().filter_map(|record| {
                let seq = record.seq?;
                (after_seq.is_none_or(|after| seq > after)).then(|| SessionLogItem::Record {
                    seq,
                    record: record.clone(),
                })
            }))
            .collect::<Vec<_>>();
        items.sort_by_key(|item| match item {
            SessionLogItem::Entry { seq, .. } | SessionLogItem::Record { seq, .. } => *seq,
        });
        if let Some(limit) = limit {
            items.truncate(limit);
        }
        items
    }

    /// Return unfinished operation starts newest-first, matching Pi's
    /// recovery query. The limit is applied after ordering.
    pub fn find_open_operations(
        &self,
        lane: &str,
        limit: Option<usize>,
    ) -> Vec<SessionLaneRecordSnapshot> {
        let mut records = self
            .lane_records
            .iter()
            .filter(|record| {
                record.record_type == "operation_started"
                    && record.lane.as_deref() == Some(lane)
                    && self.active_operations.contains_key(&record.id)
            })
            .cloned()
            .collect::<Vec<_>>();
        records.reverse();
        if let Some(limit) = limit {
            records.truncate(limit);
        }
        records
    }

    /// Decode the actor-owned operation lane without exposing mutable journal
    /// state. Admission has already validated these records; a decode error
    /// therefore indicates corrupted in-memory state and is returned instead
    /// of silently dropping a family at a consumer boundary.
    pub fn typed_lane_records(
        &self,
    ) -> Result<Vec<(SessionLaneRecordSnapshot, SessionLaneRecord)>, String> {
        self.lane_records
            .iter()
            .cloned()
            .map(|record| {
                let typed = record.typed_record()?;
                Ok((record, typed))
            })
            .collect()
    }

    /// Return every lane record through the lossless typed/opaque boundary.
    /// Known Pi families remain validated; extension records are preserved for
    /// consumers that need to inspect or round-trip them without admitting
    /// unsupported reducer semantics.
    pub fn lossless_lane_records(
        &self,
    ) -> Vec<(SessionLaneRecordSnapshot, SessionLaneRecordEnvelope)> {
        self.lane_records
            .iter()
            .cloned()
            .map(|record| {
                let envelope = record.lossless_record();
                (record, envelope)
            })
            .collect()
    }

    /// Recompute Pi's session statistics from the immutable journal.
    pub fn stats(&self) -> SessionStats {
        let mut stats = SessionStats {
            message_count: self.entries.len() as u64,
            ..SessionStats::default()
        };
        for record in &self.lane_records {
            if record.record_type != "usage" {
                continue;
            }
            let Some(usage) = record.data.get("usage") else {
                continue;
            };
            stats.cached_tokens += usage
                .get("cacheRead")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default();
            stats.uncached_tokens += usage
                .get("input")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default()
                + usage
                    .get("cacheWrite")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or_default();
            stats.total_tokens += usage
                .get("totalTokens")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or_default();
            stats.cost_total += usage
                .get("cost")
                .and_then(|cost| cost.get("total"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or_default();
        }
        stats
    }

    /// Find ordered message/config entries using Pi's EntryQuery semantics.
    pub fn find_entries(&self, query: &SessionEntryQuery) -> Vec<SessionEntryRecord> {
        let mut entries = self
            .entries
            .iter()
            .cloned()
            .map(|entry| SessionEntryRecord::Message(Box::new(entry)))
            .chain(
                self.config_records
                    .iter()
                    .cloned()
                    .map(|entry| SessionEntryRecord::Config(Box::new(entry))),
            )
            .filter(|entry| self.entry_matches_query(entry, query))
            .collect::<Vec<_>>();
        entries.sort_by_key(SessionEntryRecord::seq);
        apply_entry_order(&mut entries, query);
        entries
    }

    fn entry_matches_query(&self, entry: &SessionEntryRecord, query: &SessionEntryQuery) -> bool {
        query
            .record_type
            .as_deref()
            .is_none_or(|kind| entry.record_type() == kind)
            && query.after_seq.is_none_or(|after| entry.seq() > after)
            && query.lane.as_deref().is_none_or(|lane| match entry {
                SessionEntryRecord::Message(entry) => self.entry_lane(&entry.id) == Some(lane),
                SessionEntryRecord::Config(entry) => entry.lane == lane,
            })
            && query
                .custom_type
                .as_deref()
                .is_none_or(|custom| is_custom_entry(entry, custom))
    }

    /// Find admitted operation-lane records using Pi's ordered query rules.
    pub fn find_lane_records(&self, query: &SessionLaneQuery) -> Vec<SessionLaneRecordSnapshot> {
        let mut records = self
            .lane_records
            .iter()
            .filter(|record| {
                query
                    .lane
                    .as_deref()
                    .is_none_or(|lane| record.lane.as_deref() == Some(lane))
                    && query
                        .record_type
                        .as_deref()
                        .is_none_or(|record_type| record.record_type == record_type)
                    && query.run_id.as_deref().is_none_or(|run_id| {
                        record.data.get("runId").and_then(serde_json::Value::as_str) == Some(run_id)
                    })
                    && query.operation_kind.as_deref().is_none_or(|kind| {
                        record
                            .data
                            .get("intent")
                            .and_then(|intent| intent.get("kind"))
                            .and_then(serde_json::Value::as_str)
                            == Some(kind)
                    })
                    && query
                        .after_seq
                        .is_none_or(|after| record.seq.is_some_and(|seq| seq > after))
            })
            .cloned()
            .collect::<Vec<_>>();
        if query.newest_first {
            records.reverse();
        }
        if let Some(limit) = query.limit {
            records.truncate(limit);
        }
        records
    }

    /// Build the latest-compaction context boundary without mutating the
    /// actor-owned journal. Deferred assistant results are excluded because
    /// Pi's context builder does not send them to the provider.
    pub fn compaction_context_projection(&self) -> Option<CompactionContextProjection> {
        let compaction = self.latest_compaction()?;
        let (_, summary, retained_tail, tokens_before, timestamp) = compaction;
        let message_indices = self.compaction_message_indices(compaction.0);
        Some(CompactionContextProjection {
            summary: summary.clone(),
            tokens_before,
            timestamp,
            retained_tail: retained_tail.clone(),
            message_indices,
        })
    }

    fn latest_compaction(&self) -> Option<(u64, &String, &Vec<AgentMessage>, u64, i64)> {
        self.config_records
            .iter()
            .filter_map(|entry| match &entry.record {
                SessionConfigRecord::CompactionCreated {
                    summary,
                    retained_tail,
                    tokens_before,
                    ..
                } => Some((
                    entry.seq,
                    summary,
                    retained_tail,
                    *tokens_before,
                    entry.timestamp,
                )),
                _ => None,
            })
            .max_by_key(|(seq, ..)| *seq)
    }

    fn compaction_message_indices(&self, sequence: u64) -> Vec<usize> {
        self.entries.iter().enumerate().filter(|(_, entry)| entry.seq > sequence && !matches!(&entry.message, AgentMessage::Assistant(message) if message.stop_reason == Some(StopReason::Deferred))).map(|(index, _)| index).collect()
    }
}

