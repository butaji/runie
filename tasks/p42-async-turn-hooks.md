# p42 — Await async turn hooks

Status: in progress

Pi permits asynchronous `prepareNextTurn` and `shouldStopAfterTurn` callbacks;
the loop awaits them after `turn_end` and before steering/follow-up polling.
Runie previously exposed only synchronous hook closures.

Runie now provides async hook variants while preserving the existing sync
fields for compatibility. The driver prefers the async variant, awaits its
result, and applies the same event-driven context/model/thinking-level
projection. A deterministic integration test proves an async stop hook ends
the run before another turn is polled.
