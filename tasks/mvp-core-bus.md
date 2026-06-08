# Shared event bus with typed channels

**Status**: done

**Milestone**: MVP

**Category**: Core Architecture

## Description

Implement the shared event bus that all actors communicate through. The bus supports typed channels per actor, event tagging (domain vs ephemeral), and subscription filtering.

## Acceptance Criteria

- [x] EventBus struct with publish/subscribe methods
- [x] Typed channels per actor (each actor has its own channel)
- [x] Events tagged as domain (persisted) or ephemeral (not persisted)
- [x] Domain events: Submit, SpawnAgent, AgentThinking, AgentResponse, ToolStart, ToolEnd, Done, SwitchModel, ToolRegistered
- [x] Ephemeral events: ScrollUp, CursorLeft, CursorRight, Paste, ToggleExpand, etc.
