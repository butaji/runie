#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionHistoryRow {
    pub id: String,
    pub record_type: String,
    pub lane: String,
    pub seq: u64,
    pub parent_id: Option<String>,
    pub selected: bool,
    #[serde(default)]
    pub undoable: bool,
}

impl SessionHistoryRow {
    pub fn terminal_line(&self) -> String {
        let selected = if self.selected { "*" } else { " " };
        let parent = self.parent_id.as_deref().unwrap_or("-");
        format!(
            "{selected} {} type={} lane={} seq={} parent={parent} undoable={}",
            self.id,
            self.record_type,
            self.lane,
            self.seq,
            self.undoable
        )
    }
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
                undoable: entry.parent_id.is_some(),
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
                undoable: false,
            }))
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| row.seq);
        rows
    }

    pub fn history_rows_query(&self, query: &str) -> Vec<SessionHistoryRow> {
        let query = query.trim().to_ascii_lowercase();
        self.history_rows()
            .into_iter()
            .filter(|row| {
                [row.id.as_str(), row.record_type.as_str(), row.lane.as_str()]
                    .into_iter()
                    .any(|value| value.to_ascii_lowercase().contains(&query))
            })
            .collect()
    }
}
