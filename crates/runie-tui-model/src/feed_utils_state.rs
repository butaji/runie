use super::super::*;

/// Detect whether an `Activity` line exists between the latest user
/// message and the end of the transcript. The classifier is pure so
/// the actor-owned activity projection and the renderer share one
/// group-definition rule.
pub fn activity_group_exists_since_latest_user(snapshot: &FeedSnapshot) -> bool {
    let lines = &snapshot.lines;
    let latest_user = lines
        .iter()
        .rposition(|line| line.kind == LineKind::User)
        .unwrap_or(0);
    lines[latest_user..]
        .iter()
        .any(|line| line.kind == LineKind::Activity)
}

/// Compute the actor's `(dirs, files, commands, subagents, failures)`
/// counter tuple after a new tool starts. The `reset` flag clears the
/// counters to zero before the new tool is counted, matching the
/// `ActivityReset` event semantics in the actor's reducer.
pub fn activity_counts_with_start(
    snapshot: &FeedSnapshot,
    tool_name: &str,
    reset: bool,
) -> (usize, usize, usize, usize, usize) {
    let (mut dirs, mut files, mut commands, mut subagents, failures) = if reset {
        (0, 0, 0, 0, 0)
    } else {
        normalized_activity_counts(snapshot)
    };
    match classify_activity_tool(tool_name) {
        Some(ActivityKind::Dir) => dirs += 1,
        Some(ActivityKind::File) => files += 1,
        Some(ActivityKind::Command) => commands += 1,
        Some(ActivityKind::Subagent) => subagents += 1,
        None => {}
    }
    (dirs, files, commands, subagents, failures)
}

/// Look up the running header for a given tool call id. The helper
/// returns the most recent running tool block's header so the
/// renderer can project a stable card title.
pub fn current_tool_header(snapshot: &FeedSnapshot, tool_call_id: &str) -> Option<String> {
    snapshot
        .tool_blocks
        .iter()
        .rev()
        .find(|block| block.tool_call_id == tool_call_id && block.is_running)
        .map(|block| block.header.clone())
}

/// Look up the actor-owned args for a given tool call id. The helper
/// returns a `Null` JSON value when the args are absent so the
/// renderer can keep its optional-argument contract.
pub fn current_tool_args(snapshot: &FeedSnapshot, tool_call_id: &str) -> serde_json::Value {
    snapshot
        .facts
        .tool_args(tool_call_id)
        .cloned()
        .unwrap_or(serde_json::Value::Null)
}

/// Count the running tools in the actor-owned feed snapshot. The
/// helper is pure so the renderer and the actor agree on the
/// active-tool count for the activity fold.
pub fn active_tool_count(snapshot: &FeedSnapshot) -> usize {
    snapshot
        .tool_blocks
        .iter()
        .filter(|block| block.is_running)
        .count()
}

/// Compute the dense tool-group member positions for a `tool_ids`
/// slice. The projection returns `(member_index, group_size)` for
/// each consecutive run of `Some` entries, with `None` for the
/// separator slots. Centralized here so the actor-owned render
/// projection and the renderer agree on the dense group layout.
pub fn dense_tool_group_members(tool_ids: &[Option<&str>]) -> Vec<Option<(usize, usize)>> {
    dense_tool_group_members_by_key(tool_ids)
}

/// Compute dense-group positions using the full actor-owned member identity.
/// This keeps duplicate provider call IDs distinct when their live row IDs
/// differ, while preserving the same pure projection shape as the legacy
/// call-ID helper.
pub fn dense_tool_group_members_with_identity(
    members: &[Option<(String, Option<u64>)>],
) -> Vec<Option<(usize, usize)>> {
    dense_tool_group_members_by_key(members)
}

fn dense_tool_group_members_by_key<T: PartialEq>(
    tool_ids: &[Option<T>],
) -> Vec<Option<(usize, usize)>> {
    let mut result = vec![None; tool_ids.len()];
    let mut start = 0;
    while start < tool_ids.len() {
        if tool_ids[start].is_none() {
            start += 1;
            continue;
        }
        let mut end = start + 1;
        while end < tool_ids.len() && tool_ids[end].is_some() {
            end += 1;
        }
        let size = tool_ids[start..end]
            .iter()
            .filter(|candidate| candidate.is_some())
            .count();
        for (member_index, slot) in result[start..end].iter_mut().enumerate() {
            if tool_ids[start + member_index].is_some() {
                *slot = Some((member_index, size));
            }
        }
        start = end;
    }
    result
}

/// Project the activity counter tuple from the actor-owned feed
/// snapshot. Centralized here so the renderer and the actor share
/// one `(dirs, files, commands, subagents, failures)` shape.
pub fn activity_counts(snapshot: &FeedSnapshot) -> (usize, usize, usize, usize, usize) {
    normalized_activity_counts(snapshot)
}

fn normalized_activity_counts(snapshot: &FeedSnapshot) -> (usize, usize, usize, usize, usize) {
    (
        snapshot.facts.activity_dirs,
        snapshot.facts.activity_files,
        snapshot.facts.activity_commands,
        snapshot.facts.activity_subagents,
        snapshot.facts.activity_failures,
    )
}

/// Render a unix-timestamp (seconds) as Grok's short clock label (e.g.
/// `3:07 PM`). Falls back to a UTC-derived 12-hour clock when libc cannot
/// resolve the local timezone, so the label is always well-formed.
pub fn format_clock_timestamp(timestamp: i64) -> String {
    let (hour24, minute) = local_clock_parts(timestamp).unwrap_or_else(|| {
        const SECONDS_PER_DAY: i64 = 86_400;
        const SECONDS_PER_HOUR: i64 = 3_600;
        const SECONDS_PER_MINUTE: i64 = 60;
        let seconds = timestamp.rem_euclid(SECONDS_PER_DAY);
        (
            seconds / SECONDS_PER_HOUR,
            (seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE,
        )
    });
    let hour12 = match hour24 % 12 {
        0 => 12,
        hour => hour,
    };
    let meridiem = if hour24 < 12 { "AM" } else { "PM" };
    format!("{hour12}:{minute:02} {meridiem}")
}

/// Resolve the local 24-hour clock parts for a unix-timestamp. Returns
/// `None` when libc cannot produce a `tm` for the input (e.g. out-of-range
/// year on the host), letting callers fall back to a UTC-derived clock.
pub(crate) fn local_clock_parts(timestamp: i64) -> Option<(i64, i64)> {
    let raw = timestamp as libc::time_t;
    let mut local = std::mem::MaybeUninit::<libc::tm>::uninit();
    // SAFETY: `localtime_r` writes a complete `tm` into the valid pointer or
    // returns null. No global libc timezone state is exposed to the caller.
    let result = unsafe { libc::localtime_r(&raw, local.as_mut_ptr()) };
    if result.is_null() {
        return None;
    }
    // SAFETY: a non-null result means libc initialized the structure.
    let local = unsafe { local.assume_init() };
    Some((i64::from(local.tm_hour), i64::from(local.tm_min)))
}

/// Append the streaming tool-update fragment to a retained tool header. The
/// serialized partial result is the transport payload verbatim; a payload that
/// cannot be serialized degrades to an empty fragment so the header stays
/// well-formed.
pub fn tool_update_header_text(current_header: &str, partial_result: &serde_json::Value) -> String {
    format!(
        "{current_header} | update: {}",
        serde_json::to_string(partial_result).unwrap_or_default()
    )
}
