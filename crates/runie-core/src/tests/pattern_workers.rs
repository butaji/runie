//! Swarm pattern worker lifecycle rows: events → state registry → feed elements.

use crate::event::{Event, EventCategory, EventKind};
use crate::model::{AppState, PatternWorkerStatus};
use crate::tests::fresh_state;
use crate::view::elements::{Element, PostKind};
use crate::view::LazyCache;

fn spawn(state: &mut AppState, id: &str, desc: &str, model: &str) {
    state.update(Event::PatternWorkerSpawned {
        id: id.into(),
        description: desc.into(),
        model: model.into(),
    });
}

fn finish(state: &mut AppState, id: &str, status: &str, duration_ms: u64, output: &str) {
    state.update(Event::PatternWorkerFinished {
        id: id.into(),
        status: status.into(),
        duration_ms,
        output: output.into(),
    });
}

fn subagent_rows(state: &AppState) -> Vec<Element> {
    LazyCache::feed(state)
        .elements
        .into_iter()
        .filter(|e| matches!(e, Element::SubagentRow { .. }))
        .collect()
}

// ── Event taxonomy ───────────────────────────────────────────────────────────

#[test]
fn pattern_worker_events_are_agent_facts() {
    let spawned = Event::PatternWorkerSpawned {
        id: "w.1".into(),
        description: "d".into(),
        model: "m".into(),
    };
    let finished = Event::PatternWorkerFinished {
        id: "w.1".into(),
        status: "completed".into(),
        duration_ms: 1,
        output: "o".into(),
    };
    for e in [spawned, finished] {
        assert_eq!(e.kind(), EventKind::Fact, "{e:?} kind");
        assert_eq!(e.category(), EventCategory::Agent, "{e:?} category");
        assert!(
            e.clone().into_intent().is_none(),
            "{e:?} must not be an intent"
        );
    }
}

#[test]
fn pattern_worker_events_are_transient_not_durable() {
    let spawned = Event::PatternWorkerSpawned {
        id: "w.1".into(),
        description: "d".into(),
        model: "m".into(),
    };
    let finished = Event::PatternWorkerFinished {
        id: "w.1".into(),
        status: "completed".into(),
        duration_ms: 1,
        output: "o".into(),
    };
    assert!(spawned.to_durable().is_none());
    assert!(finished.to_durable().is_none());
}

// ── State registry ───────────────────────────────────────────────────────────

#[test]
fn spawned_event_creates_running_row() {
    let mut state = fresh_state();
    spawn(&mut state, "w.1", "Summarize the task", "echo");

    let workers = &state.agent_state().pattern_workers;
    assert_eq!(workers.len(), 1);
    let row = &workers[0];
    assert_eq!(row.id, "w.1");
    assert_eq!(row.description, "Summarize the task");
    assert_eq!(row.model, "echo");
    assert_eq!(row.status, PatternWorkerStatus::Running);
    assert!(row.duration_ms.is_none());
    assert!(row.output.is_empty());
}

#[test]
fn spawned_same_id_replaces_existing_row() {
    let mut state = fresh_state();
    spawn(&mut state, "w.1", "first", "echo");
    spawn(&mut state, "w.1", "second", "echo");

    let workers = &state.agent_state().pattern_workers;
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].description, "second");
    assert_eq!(workers[0].status, PatternWorkerStatus::Running);
}

#[test]
fn finished_completed_updates_row_in_place() {
    let mut state = fresh_state();
    spawn(&mut state, "w.1", "Summarize the task", "echo");
    finish(&mut state, "w.1", "completed", 2500, "worker output");

    let workers = &state.agent_state().pattern_workers;
    assert_eq!(workers.len(), 1);
    let row = &workers[0];
    assert_eq!(row.status, PatternWorkerStatus::Completed);
    assert_eq!(row.duration_ms, Some(2500));
    assert_eq!(row.output, "worker output");
}

#[test]
fn finished_failed_marks_row_failed() {
    let mut state = fresh_state();
    spawn(&mut state, "w.1", "Summarize the task", "echo");
    finish(&mut state, "w.1", "failed", 900, "boom");

    let row = &state.agent_state().pattern_workers[0];
    assert_eq!(row.status, PatternWorkerStatus::Failed);
    assert_eq!(row.output, "boom");
}

#[test]
fn finished_cancelled_marks_row_cancelled() {
    let mut state = fresh_state();
    spawn(&mut state, "w.1", "Summarize the task", "echo");
    finish(&mut state, "w.1", "cancelled", 100, "");

    assert_eq!(
        state.agent_state().pattern_workers[0].status,
        PatternWorkerStatus::Cancelled
    );
}

#[test]
fn finished_unknown_status_marks_row_failed() {
    let mut state = fresh_state();
    spawn(&mut state, "w.1", "Summarize the task", "echo");
    finish(&mut state, "w.1", "exploded", 100, "");

    assert_eq!(
        state.agent_state().pattern_workers[0].status,
        PatternWorkerStatus::Failed
    );
}

#[test]
fn finished_output_capped_at_20000_chars() {
    let mut state = fresh_state();
    spawn(&mut state, "w.1", "Summarize the task", "echo");
    let big = "a".repeat(25_000);
    finish(&mut state, "w.1", "completed", 1, &big);

    assert_eq!(state.agent_state().pattern_workers[0].output.len(), 20_000);
}

#[test]
fn turn_started_clears_pattern_workers() {
    let mut state = fresh_state();
    spawn(&mut state, "w.1", "Summarize the task", "echo");
    assert_eq!(state.agent_state().pattern_workers.len(), 1);

    state.update(Event::TurnStarted {
        id: "req.1".into(),
        request_id: "req.1".into(),
        content: "next turn".into(),
    });
    assert!(state.agent_state().pattern_workers.is_empty());
}

// ── Transform ────────────────────────────────────────────────────────────────

#[test]
fn transform_injects_subagent_row_per_worker() {
    let mut state = fresh_state();
    spawn(&mut state, "w.1", "Summarize the task", "echo");
    spawn(&mut state, "w.2", "Draft an outline", "echo");

    let rows = subagent_rows(&state);
    assert_eq!(rows.len(), 2);
    let descriptions: Vec<&str> = rows
        .iter()
        .map(|e| match e {
            Element::SubagentRow { description, .. } => description.as_str(),
            _ => unreachable!(),
        })
        .collect();
    assert!(descriptions.contains(&"Summarize the task"));
    assert!(descriptions.contains(&"Draft an outline"));
}

#[test]
fn running_subagent_row_sorts_above_thinking_row() {
    let mut state = fresh_state();
    state.set_streaming(true);
    state.update(Event::Thinking { id: "req.0".into() });
    spawn(&mut state, "w.1", "Summarize the task", "echo");

    let feed = LazyCache::feed(&state);
    let sub_idx = feed
        .elements
        .iter()
        .position(|e| matches!(e, Element::SubagentRow { .. }))
        .expect("subagent row present");
    let thinking_idx = feed
        .elements
        .iter()
        .position(|e| matches!(e, Element::Thinking { .. }))
        .expect("thinking row present");
    assert!(
        sub_idx < thinking_idx,
        "worker row must sort above the thinking row: {sub_idx} vs {thinking_idx}"
    );
}

#[test]
fn completed_row_is_collapsible_post_running_row_is_not() {
    let mut state = fresh_state();
    spawn(&mut state, "w.1", "Summarize the task", "echo");
    spawn(&mut state, "w.2", "Draft an outline", "echo");
    finish(&mut state, "w.2", "completed", 100, "done");

    let feed = LazyCache::feed(&state);
    let mut running_post = None;
    let mut completed_post = None;
    for post in &feed.posts {
        let is_subagent = feed.elements[post.start..post.end]
            .iter()
            .any(|e| matches!(e, Element::SubagentRow { .. }));
        if !is_subagent {
            continue;
        }
        assert_eq!(post.kind, PostKind::SubagentRow);
        let elem = &feed.elements[post.start];
        match elem {
            Element::SubagentRow {
                status: PatternWorkerStatus::Running,
                ..
            } => running_post = Some(post),
            Element::SubagentRow {
                status: PatternWorkerStatus::Completed,
                ..
            } => completed_post = Some(post),
            _ => {}
        }
    }
    let running_post = running_post.expect("running worker post present");
    let completed_post = completed_post.expect("completed worker post present");
    assert!(
        running_post.expanded,
        "running rows are not collapsible (nothing to expand)"
    );
    assert!(
        !completed_post.expanded,
        "completed rows collapse to the one-line summary by default"
    );
}

#[test]
fn expanding_completed_post_reveals_worker_output() {
    let mut state = fresh_state();
    spawn(&mut state, "w.1", "Summarize the task", "echo");
    finish(&mut state, "w.1", "completed", 100, "line one\nline two");

    let feed = LazyCache::feed(&state);
    let post_idx = feed
        .posts
        .iter()
        .position(|p| {
            feed.elements[p.start..p.end]
                .iter()
                .any(|e| matches!(e, Element::SubagentRow { .. }))
        })
        .expect("subagent post present");
    state.view_mut().expanded_posts.insert(post_idx);

    let rows = subagent_rows(&state);
    assert_eq!(rows.len(), 1);
    match &rows[0] {
        Element::SubagentRow {
            expanded, output, ..
        } => {
            assert!(expanded, "post in expanded_posts must render expanded");
            assert_eq!(output, "line one\nline two");
        }
        _ => unreachable!(),
    }
}
