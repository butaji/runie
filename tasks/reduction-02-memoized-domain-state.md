# Reduction 02: memoized domain state

Status: adopted

Move state derivation toward `state = memo { reduce(events) }`. Preserve event
ordering, replay, and actor ownership while removing imperative projection
duplication.

Progress: `runie_core::EventMemo` now provides immutable event retention,
incremental reduction, and full replay equivalence without async coupling.
`ScrollbackActor` now uses it as the canonical feed reducer state. `StatusActor`
now uses it for status snapshot reduction as well.

Remaining adoption work is tracked by reductions 03, 04, 06, and 08.

Acceptance: reducer purity tests, replay equivalence, and no direct
cross-actor mutation.
