# p39 — Typed event-sequence DSL

Status: in progress

Runie now provides `event_sequence!` as a deliberately small Rust-side DSL
for constructing owned event vectors. It supports literal event expressions
and repeated cloned events, with no reducer, actor access, or side effects.

YAML remains the authoritative fixture format for replay and TUI parity: YAML
changes are loaded at runtime and do not require recompiling the runner. The
macro is limited to compact typed tests and adapters where Rust expressions
are already required.

Future DSL work must preserve this separation: declarative event data first,
actor-owned reduction second, pure snapshot rendering third.
