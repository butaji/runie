# Reduction 07: state/widget separation

Status: adopted

Actors own model state; widgets consume immutable snapshots and only render.
Remove widget objects from actor state where they duplicate model ownership.

Verified in the current TUI boundary: prompt, scrollback, and status actors
publish renderer-independent snapshots; `from_model_snapshot` constructs
widgets only at the renderer-facing API. Reducer workers retain mutable state
inside their owning actors.

Acceptance: renderer purity tests and unchanged key/event behavior.
