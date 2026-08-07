# p41 — Reject reset during an active Pi run

Status: in progress

Pi's `Agent.reset()` throws while an active run exists. Runie's previous reset
path published `Reset` and cleared queues unconditionally, allowing a reset to
race provider streaming and violate the Pi lifecycle contract.

`LoopActor::reset()` now owns the single run-admission permit through the reset
event and returns `LoopError::Busy` when another run owns it. The TUI forwards
the result without mutating projections directly. A deterministic
blocking-stream integration test proves the active-run rejection and normal
completion after release; it uses watch channels and no sleeps.
