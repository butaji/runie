# p18 — Architecture: actors-ssot audit, event ownership, no orphan spawns, linter rules

**Parity target:** clean architecture invariant (project AGENTS.md) enforced across all p01-p17 changes.

## Reference

- pi event contract this architecture must preserve: `agent-loop.ts:95-274` (event ordering), `proxy.ts` (streaming), `agent.ts` (state ownership) — see `~/Code/agents/pi/packages/agent/src/`.
- Project `AGENTS.md` (repo root): events-based single-source-of-truth actors; each state slice owned by exactly one actor; the only change mechanism is events published by the owner; handlers/tools/tests never mutate another actor's state directly; read-only projections rebuilt from events; every spawned task has an owner (`JoinHandle`/`JoinSet`/completion event), no orphan `tokio::spawn`.
- `lint-check/src/main.rs` enforces across `crates/runie-core/src/**.rs`: magic numbers `>= 1000` (named constants; exempt <1000, underscore, hex, HTTP/JSON-RPC codes, test code) and orphan `tokio::spawn`. Keep files ~400 lines, functions ~60 lines.

## Adapt to runie

1. Re-audit every new actor introduced by p05/p06/p07 (loop continuation, busy guard, turn hooks) and p12 (state projections) for:
   - single owner per state slice;
   - events as the only mutation mechanism;
   - projections rebuilt from events, not stored state.
2. Confirm every `tokio::spawn` in the new code stores its handle (JoinSet/JoinHandle) or is joined; no orphans (the `ProviderActor`, `ToolExecutorActor`, `QueueActor`s, `LoopActor`, recorder tasks already follow this — keep it).
3. Run `cargo run -p lint-check` — must be clean after all changes.
4. Keep `types.rs`/`driver.rs`/`executor.rs` under the ~400-line target; split files if they exceed it (e.g. split `driver.rs` if auto-continue/hooks grow it).
5. Update `AGENTS.md` if any new actor/convention (e.g. turn hooks, state projection rules) needs documenting.

## State machine / variants

Audit checklist per module (pass/fail):
- `loop/driver.rs` — loop owns run state; continuation via events; no direct cross-actor mutation.
- `state/actor.rs` — owns the state slice; projections pure.
- `queues/*` — each queue actor owns its queue; drained via commands.
- `provider/*`, `tools/*` — mailbox actors; spawned tasks owned/joined.
- `events/*` — single bus; subscribers read-only.

## Acceptance

- `cargo run -p lint-check` clean.
- `cargo clippy --workspace --all-targets` clean (fix pre-existing warnings this sweep touches).
- `cargo fmt --all -- --check` clean.
- `cargo check --workspace --all-targets` green.