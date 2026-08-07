# p39 — Typed event-sequence DSL

Status: complete (2026-08-07)

Runie now provides `event_sequence!` as a deliberately small Rust-side DSL
for constructing owned event vectors. It supports literal event expressions
and repeated cloned events, with no reducer, actor access, or side effects.

YAML remains the authoritative fixture format for replay and TUI parity: YAML
changes are loaded at runtime and do not require recompiling the runner. The
macro is limited to compact typed tests and adapters where Rust expressions
are already required.

Future DSL work must preserve this separation: declarative event data first,
actor-owned reduction second, pure snapshot rendering third.

Completion evidence: the macro has focused unit coverage, the YAML runner
discovers and executes the runtime fixtures without recompilation, and the
canonical `visual-hey.yaml` scenario asserts ordered Runie, Pi, and awaited
listener event traces before its state and visual assertions.

## Session mapping DSL increment (2026-08-07)

The session actor now has one pure `session_config_record!` mapping table for
all application events that become journal facts. Both the live bus bridge and
the replay `apply_event` path use it before sending a record through the actor
mailbox. This removes duplicated event classification while keeping YAML as
the runtime-editable behavioral surface and preserving exhaustive no-op
classification for non-session events.
