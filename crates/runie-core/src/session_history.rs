#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionHistoryRow {
    pub id: String,
    pub record_type: String,
    pub lane: String,
    pub seq: u64,
    pub parent_id: Option<String>,
    pub selected: bool,
}

impl SessionSnapshot {
    /// Project the journal into stable picker/history data. The selected
    /// branch is marked without discarding alternate undo targets.
    pub fn history_rows(&self) -> Vec<SessionHistoryRow> {
        let selected = self
            .branch_entry_ids()
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();
        let mut rows = self
            .entries
            .iter()
            .map(|entry| SessionHistoryRow {
                id: entry.id.clone(),
                record_type: "message".into(),
                lane: entry.lane.clone(),
                seq: entry.seq,
                parent_id: entry.parent_id.clone(),
                selected: selected.contains(&entry.id),
            })
            .chain(self.config_records.iter().map(|entry| SessionHistoryRow {
                id: entry.id.clone(),
                record_type: SessionEntryRecord::Config(Box::new(entry.clone()))
                    .record_type()
                    .into(),
                lane: entry.lane.clone(),
                seq: entry.seq,
                parent_id: entry.parent_id.clone(),
                selected: selected.contains(&entry.id),
            }))
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.seq);
        rows
    }
}
