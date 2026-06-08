# Session persistence with event log

**Status**: todo

**Milestone**: MVP

**Category**: Core Architecture

## Description

Persist sessions as event logs. Save domain events to JSONL files, replay on load.

## Acceptance Criteria

- [ ] Domain events serialized to JSONL
- [ ] Session load replays events into all actors
- [ ] SessionManager handles save/load/list/delete
- [ ] Periodic snapshots as load accelerators
