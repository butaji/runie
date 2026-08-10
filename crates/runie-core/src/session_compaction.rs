fn project_compacted_context(
    messages: Vec<(u64, AgentMessage)>,
    compaction_seq: u64,
    timestamp: i64,
    summary: String,
    retained_tail: Vec<AgentMessage>,
    tokens_before: u64,
) -> Vec<AgentMessage> {
    let mut projected = vec![AgentMessage::CompactionSummary(
        crate::types::CompactionSummaryMessage {
            summary,
            tokens_before,
            timestamp,
        },
    )];
    projected.extend(
        retained_tail
            .into_iter()
            .filter(is_provider_context_message),
    );
    projected.extend(
        messages
            .into_iter()
            .filter(|(seq, _)| *seq > compaction_seq)
            .map(|(_, message)| message),
    );
    projected
}

/// Build the pure payload for Pi's async compaction owner. Only index
/// selection happens here; summary generation and journal publication remain
/// separate event-driven operations.
pub fn prepare_compaction_entries(
    entries: &[SessionEntry],
    token_estimates: &[u64],
    keep_recent_tokens: u64,
) -> Result<Option<CompactionPreparation>, String> {
    if entries.is_empty() {
        return Ok(None);
    }
    let cut_point = find_compaction_cut_point(
        entries,
        token_estimates,
        0,
        entries.len(),
        keep_recent_tokens,
    )?;
    let history_end = cut_point
        .turn_start_index
        .unwrap_or(cut_point.first_kept_entry_index);
    Ok(Some(CompactionPreparation {
        history_indices: (0..history_end).collect(),
        turn_prefix_indices: cut_point
            .turn_start_index
            .map(|start| (start..cut_point.first_kept_entry_index).collect())
            .unwrap_or_default(),
        retained_indices: (cut_point.first_kept_entry_index..entries.len()).collect(),
        tokens_before: token_estimates.iter().sum(),
        cut_point,
        source_entry_ids: entries.iter().map(|entry| entry.id.clone()).collect(),
    }))
}

/// Select Pi's recent-context cut point without performing summarization.
/// `token_estimates` is supplied by the caller so estimation policy stays
/// explicit and testable; entries that cannot begin a turn (tool results) are
/// never selected as a cut point.
pub fn find_compaction_cut_point(
    entries: &[SessionEntry],
    token_estimates: &[u64],
    start_index: usize,
    end_index: usize,
    keep_recent_tokens: u64,
) -> Result<CompactionCutPoint, String> {
    validate_cut_point_bounds(entries, token_estimates, start_index, end_index)?;
    let cut_points = eligible_cut_points(entries, start_index, end_index);
    let Some(mut cut_index) = cut_points.first().copied() else {
        return Ok(CompactionCutPoint {
            first_kept_entry_index: start_index,
            turn_start_index: None,
            is_split_turn: false,
        });
    };
    cut_index = recent_cut_index(
        entries,
        token_estimates,
        start_index,
        end_index,
        keep_recent_tokens,
        &cut_points,
        cut_index,
    );
    let is_user = matches!(&entries[cut_index].message, AgentMessage::User(_));
    let turn_start_index = find_turn_start(entries, start_index, cut_index, is_user);
    Ok(CompactionCutPoint {
        first_kept_entry_index: cut_index,
        turn_start_index,
        is_split_turn: turn_start_index.is_some(),
    })
}

fn validate_cut_point_bounds(
    entries: &[SessionEntry],
    token_estimates: &[u64],
    start_index: usize,
    end_index: usize,
) -> Result<(), String> {
    if start_index > end_index
        || end_index > entries.len()
        || entries.len() != token_estimates.len()
    {
        return Err("compaction cut-point bounds do not match entries".into());
    }
    Ok(())
}

fn eligible_cut_points(
    entries: &[SessionEntry],
    start_index: usize,
    end_index: usize,
) -> Vec<usize> {
    (start_index..end_index)
        .filter(|index| {
            matches!(
                &entries[*index].message,
                AgentMessage::User(_) | AgentMessage::Assistant(_)
            )
        })
        .collect()
}

fn recent_cut_index(
    entries: &[SessionEntry],
    token_estimates: &[u64],
    start_index: usize,
    end_index: usize,
    keep_recent_tokens: u64,
    cut_points: &[usize],
    mut cut_index: usize,
) -> usize {
    let mut accumulated = 0;
    for index in (start_index..end_index).rev() {
        if !matches!(&entries[index].message, AgentMessage::ToolResult(_)) {
            accumulated += token_estimates[index];
        }
        if accumulated >= keep_recent_tokens {
            if let Some(candidate) = cut_points
                .iter()
                .copied()
                .find(|candidate| *candidate >= index)
            {
                cut_index = candidate;
            }
            break;
        }
    }
    cut_index
}

fn find_turn_start(
    entries: &[SessionEntry],
    start_index: usize,
    cut_index: usize,
    is_user: bool,
) -> Option<usize> {
    (!is_user).then(|| {
        (start_index..=cut_index)
            .rev()
            .find(|index| matches!(&entries[*index].message, AgentMessage::User(_)))
    })?
}

/// Pi's durable operation-lane record families. The payload remains JSON at
/// the wire boundary, but classification is typed before the actor reducer
/// changes its owned projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionLaneRecordKind {
    OperationStarted,
    AbortRequested,
    OperationFinished,
    StepAttempt,
    ToolStarted,
    QueueEnqueued,
    QueueCancelled,
    WriteDeferred,
    Usage,
}
