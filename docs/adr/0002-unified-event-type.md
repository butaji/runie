# Unified event type in runie-core

All events — terminal input, agent lifecycle, tool execution — use a single `Event` type defined in `runie-core`. There is no separate `AgentEvent` type with conversion logic.

This replaces the previous model where `runie-agent` defined `AgentEvent` with a `to_core_event()` conversion to `runie-core::Event`.

Events are tagged as ephemeral or domain. Ephemeral events (ScrollUp, CursorLeft, etc.) are not persisted. Domain events (Submit, AgentResponse, ToolEnd, etc.) are logged to the session.
