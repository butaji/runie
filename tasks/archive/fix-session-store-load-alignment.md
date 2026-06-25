# Fix `SessionStore::load_events` misalignment and silent drops

**Status**: done
**Milestone**: R3
**Category**: Sessions
**Priority**: P1

**Depends on**: none
**Blocks**: none

## Description

`crates/runie-core/src/session_store.rs:180-193` reads session events from redb into two separate vectors (`keys` and `events`). When a row fails to deserialize, the event is silently dropped but its key remains. The subsequent `keys.into_iter().zip(events)` then pairs later keys with earlier events, corrupting the event order and dropping data. The fix is to pair key and event inside the loop and surface parse failures.

## Acceptance Criteria

- [ ] Malformed rows no longer shift subsequent events out of order.
- [ ] A parse failure is logged or returned as an error instead of being silent.
- [ ] `cargo test --workspace` passes.

## Tests

### Layer 1 — State/Logic
- [ ] Add `load_events_rejects_misaligned_entries` in `crates/runie-core/src/tests/session_store.rs`:
  - Create a redb store with three events.
  - Manually corrupt the middle JSON value in the table.
  - Call `load_events` and assert it returns an `Err` (or at least does not silently swap events).
- [ ] Add `load_events_returns_ordered_events` to verify normal ordering.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Provider Replay / E2E
- N/A.

## Files touched

- `crates/runie-core/src/session_store.rs`
- `crates/runie-core/src/tests/session_store.rs`

## Implementation

### Step 1: Pair key and event inside the loop

Replace the current loop and zip/sort with:

```rust
let mut paired = Vec::new();
let mut parse_errors = Vec::new();

for entry in table.iter()? {
    let (k, v) = entry?;
    let key = k.value();
    let val = v.value();
    match serde_json::from_str::<DurableCoreEvent>(val) {
        Ok(event) => paired.push((key, event)),
        Err(e) => {
            tracing::warn!("failed to parse session event at key {}: {}", key, e);
            parse_errors.push(format!("key {}: {}", key, e));
        }
    }
}

if !parse_errors.is_empty() {
    return Err(anyhow::anyhow!(
        "session {} contains {} unparseable event(s): {}",
        session_id,
        parse_errors.len(),
        parse_errors.join("; ")
    ));
}

paired.sort_by_key(|(k, _)| *k);
let events: Vec<_> = paired.into_iter().map(|(_, e)| e).collect();
Ok(events)
```

### Step 2: Update JSONL fallback similarly

In `load_events_jsonl`, also collect parse errors or at least log them. Currently it silently skips empty lines but not parse errors. For consistency, change the line loop to:

```rust
for line in reader.lines() {
    let line = line?;
    if line.trim().is_empty() {
        continue;
    }
    match serde_json::from_str::<DurableCoreEvent>(&line) {
        Ok(event) => events.push(event),
        Err(e) => parse_errors.push(format!("line: {e}")),
    }
}
```

Return an error if `parse_errors` is non-empty.

### Step 3: Add tests

```rust
fn sample_message(id: &str, content: &str) -> DurableCoreEvent {
    DurableCoreEvent::MessageSent {
        id: id.into(),
        role: "user".into(),
        content: content.into(),
        timestamp: 0.0,
        provider: "".into(),
    }
}

#[test]
fn load_events_returns_ordered_events() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SessionStore::new(tmp.path().to_path_buf());
    let events = vec![
        sample_message("a", "a"),
        sample_message("b", "b"),
    ];
    for e in &events {
        store.append("s1", e).unwrap();
    }
    let loaded = store.load_events("s1").unwrap();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[0], events[0]);
    assert_eq!(loaded[1], events[1]);
}

#[test]
fn load_events_rejects_misaligned_entries() {
    let tmp = tempfile::tempdir().unwrap();
    let store = SessionStore::new(tmp.path().to_path_buf());
    let good = sample_message("a", "a");
    store.append("s1", &good).unwrap();

    // Manually corrupt the second row.
    let path = store.path("s1");
    let (db, _) = SessionStore::open_db(&path).unwrap();
    let tx = db.begin_write().unwrap();
    {
        let mut table = tx.open_table(TABLE_EVENTS).unwrap();
        table.insert(&1, "not json").unwrap();
    }
    tx.commit().unwrap();

    assert!(store.load_events("s1").is_err());
}
```

### Step 4: Run tests

```bash
cargo test -p runie-core session_store
cargo test --workspace
```

### Step 5: Commit

```bash
git add crates/runie-core/src/session_store.rs crates/runie-core/src/tests/session_store.rs tasks/fix-session-store-load-alignment.md tasks/index.json
git commit -m "fix(core): align session store keys with events and surface parse errors"
```

## Notes

- Returning an error changes behavior from silent data loss to a hard failure. This is safer; if partial loading is desired, add a config flag in a follow-up task.
- Ensure `tracing` is imported in `session_store.rs`.
