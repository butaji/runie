use super::super::*;

pub fn apply_entry_order(entries: &mut Vec<SessionEntryRecord>, query: &SessionEntryQuery) {
    if query.newest_first {
        entries.reverse();
    }
    if let Some(limit) = query.limit {
        entries.truncate(limit);
    }
}

pub fn is_custom_entry(entry: &SessionEntryRecord, custom_type: &str) -> bool {
    matches!(entry, SessionEntryRecord::Config(entry) if matches!(&entry.record, SessionConfigRecord::CustomSessionEntryCreated { custom_type: value, .. } if value == custom_type))
}

crate::wire_kind! {
    pub enum SessionOperationKind {
        Started => "operation_started",
        Finished => "operation_finished",
        AbortRequested => "abort_requested",
    }
}
