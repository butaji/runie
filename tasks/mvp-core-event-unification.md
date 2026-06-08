# Unified event type in runie-core

**Status**: done

**Milestone**: MVP

**Category**: Core Architecture

## Description

Unify all events into a single Event enum in runie-core. Remove the separate AgentEvent type with conversion logic.

## Acceptance Criteria

- [x] Single Event enum in runie-core::event
- [x] No AgentEvent type in runie-agent
- [x] All terminal input events included
- [x] All agent lifecycle events included
- [x] All tool events included
