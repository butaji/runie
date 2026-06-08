# Orchestrator spawning all actors

**Status**: done

**Milestone**: MVP

**Category**: Core Architecture

## Description

Implement the Orchestrator as the central spawn point for all actors. The Orchestrator holds typed senders to each actor and routes messages accordingly.

## Acceptance Criteria

- [x] Orchestrator spawns AgentLoop, QueueAgent, SessionManager, ConfigAgent
- [x] Orchestrator holds typed channels to each actor
- [x] ToolActors spawned via Orchestrator on ToolStart events
