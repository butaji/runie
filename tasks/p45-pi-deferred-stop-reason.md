# p45 — Pi deferred stop-reason parity

Status: implemented (2026-08-06)

Pi's `StopReason` union includes `deferred` for providers that return a
deferred response handle. Runie previously covered `stop`, `length`,
`toolUse`, `error`, and `aborted`, but rejected this valid Pi value at the
serde/type boundary.

The `StopReason::Deferred` variant and typed `DeferredHandle` now survive core
serialization and status projection. `StopReasonSpec::Deferred` plus the
optional handle expose the same value in the runtime YAML event DSL, and
`deferred-response.yaml` drives it through the real loop, event bus, actor
state projection, and Pi event oracle without recompilation.

This slice preserves the event payload rather than inventing deferred-fetch
behavior; actual provider deferred-handle APIs remain a separate adapter
contract requiring source-backed transport semantics.

Verification: focused stop-reason and status tests plus the full `just ci`
suite are required for each future change to this contract.
