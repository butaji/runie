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

The YAML runner now also exposes the effective steering and follow-up queue
policies in `assertions.state`; `follow-up.yaml` verifies the `all` policy
after actor construction. This makes queue configuration a state assertion,
not merely a deserialization check.

Navigation ordering (2026-08-06): fold and tool-selection declarations now
reduce in their original YAML order through the scrollback actor. Previously
the runner grouped folds before selections, which could hide ordering bugs in
selection → fold sequences. This keeps YAML as the executable event sequence
oracle while preserving the actor/reducer boundary.
