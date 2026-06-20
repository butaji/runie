# Split runie-core into domain and IO crates

**Status**: todo
**Milestone**: R4
**Category**: Architecture / Actors
**Priority**: P0

**Depends on**: finish-io-migration, delete-async-io-bridge, fold-state-into-model-state, rename-core-ui-to-view
**Blocks**: gate-or-move-single-consumer-core-modules, unify-duplicate-module-names-core-tui, consolidate-config-modules-into-dir

## Description

`runie-core` is a 51,524-line / 336-file god-crate with 30+ direct deps that mixes all three layers the `docs/Architecture.md:9-13` claims to separate:

- **IO layer in core**: `actors/` (config/, fff_indexer/, io/, persistence/, provider/, session_store/), `session_store.rs` (`redb`), `auth.rs`, `clipboard_image.rs` (`arboard`+`png`), `mcp.rs`, `actors/io/git.rs` (`git2`), `actors/config/actor.rs` (`notify`).
- **UI DSL in core**: `ui/` (1,040 LOC: `elements.rs`, `transform.rs`, `posts.rs`).
- **Domain (the only pure part)**: `model/`, `event/`, `update/`.

Every other crate depends on `runie-core`, so `runie-print` (95-line binary) transitively compiles `redb`, `git2`, `notify`, `arboard`, `png`, `tiktoken-rs`, `fff-search`, `schemars`, `jsonschema`, `pulldown-cmark`, `nucleo-matcher`, `textwrap`. The "No synchronous IO lives in the domain crate" rule is enforced only by a source-scan allow-list, not by crate boundaries.

`finish-io-migration` explicitly deferred this split ("The larger question of splitting runie-core into runie-domain + runie-io crates is deferred"). Its prerequisites (sync IO behind traits, `async_io.rs` bridge deleted, state trees folded, `ui/` renamed to `view/`) are now tracked by individual tasks. This task performs the actual crate split once they land.

Proposed split:

| New crate | Contents | Deps |
|-----------|----------|------|
| `runie-domain` | `model/`, `event/`, `update/`, `view/` (pure view-model), `snapshot.rs`, `message/`, `permissions/` (rules, no IO), `commands/` (DSL), `dialog/` (DSL), `llm_event.rs`, `tool/` (trait+registry types), `tool_runtime.rs` (trait), `harness_skills/` (trait), `labels.rs`, `display_width.rs`, `diff.rs` (pure), `markdown/` (pure parsing) | `serde`, `anyhow`, `similar`, `pulldown-cmark`, `unicode-width`, `textwrap` |
| `runie-io` | `actors/` (all impls), `session_store.rs`, `auth.rs`, `clipboard_image.rs`, `mcp.rs`, `async_io.rs` (until deleted), `config_reload/`, `headless_runtime.rs` (concrete), `ipc.rs` (if not folded into server) | `runie-domain`, `redb`, `git2`, `notify`, `arboard`, `png`, `fff-search`, `tiktoken-rs`, `tokio`, `reqwest` |

`runie-protocol` is handled by `fold-protocol-into-core`; `runie-engine` by `fold-runie-engine-into-agent`. This task assumes those are resolved (or coordinates: protocol folds into whichever crate owns `ipc.rs`; engine folds into agent).

## Acceptance Criteria

- [ ] `crates/runie-domain/` exists with only pure modules (no `std::fs`, no `tokio::process`, no `reqwest`, no `redb`, no `git2`, no `notify`, no `arboard`, no `png`).
- [ ] `crates/runie-io/` exists with all actor implementations and concrete stores.
- [ ] `crates/runie-domain/Cargo.toml` deps are a strict subset of current `runie-core` deps excluding every IO/heavy dep listed above.
- [ ] `runie-core` is deleted; or kept as a facade re-exporting `runie-domain` + `runie-io` for one release cycle (decide explicitly).
- [ ] `runie-print` and `runie-json` depend only on `runie-domain` (+ `runie-agent`, `runie-provider`); they no longer transitively compile `redb`/`git2`/`notify`/`arboard`/`png`/`tiktoken-rs`/`fff-search`/`schemars`/`jsonschema`.
- [ ] `cargo build --release -p runie-print` compiles measurably faster than before (record before/after wall time).
- [ ] `arch_guardrails.rs` enforces "no `std::fs`/`tokio::process`/`reqwest` in `runie-domain`" with zero allow-list entries.
- [ ] `cargo test --workspace` succeeds.
- [ ] `cargo check --workspace` succeeds with no new warnings.

## Tests

### Layer 1 — State/Logic
- [ ] `domain_compiles_without_io_deps` — `cargo check -p runie-domain` succeeds with `--no-default-features` (no IO features).
- [ ] All existing `model/`, `event/`, `update/` state tests pass unchanged after the move.

### Layer 2 — Event Handling
- [ ] All existing event-handling tests pass unchanged (events live in domain).

### Layer 3 — Rendering
- [ ] `view_module_renders_in_test_backend` — `view/` (renamed from `ui/`) TestBackend tests pass from the new crate location.

### Layer 4 — Smoke / Crash
- [ ] `smoke_print_binary_no_io_deps` — `cargo tree -p runie-print` shows no `redb`/`git2`/`notify`/`arboard`/`png`/`tiktoken-rs`/`schemars`/`jsonschema` edges.
- [ ] `smoke_tui_actor_wiring_intact` — full TUI bootstrap still spawns all actors from the new crate layout.
- [ ] `arch_test_domain_has_no_io` — guardrail test scans `crates/runie-domain/src` for `std::fs::`/`tokio::process::`/`reqwest::` and fails on any hit.

## Files touched

- `Cargo.toml` (workspace members: replace `runie-core` with `runie-domain` + `runie-io`; or add both and deprecate core)
- `crates/runie-domain/` (new crate; `git mv` of pure modules from `runie-core/src/`)
- `crates/runie-io/` (new crate; `git mv` of actor/store/IO modules from `runie-core/src/`)
- `crates/runie-core/build.rs` → moves to `runie-domain/build.rs` (the 500/40/10 guardrails apply to domain; io crate may need relaxed function-length for actor `run()` bodies)
- Every `Cargo.toml` that declares `runie-core` → update to `runie-domain` and/or `runie-io` as appropriate
- `crates/runie-core/tests/arch_guardrails.rs` → split into `runie-domain/tests/arch_guardrails.rs` (no-IO rule) + `runie-io/tests/` (actor invariants)
- `docs/Architecture.md` crate map updated

## Notes

This is the highest-leverage move in the audit: it makes the "IO | Domain (pure) | UI" posture true at the crate level instead of via a source-scan allow-list. It subsumes the framing of `gate-or-move-single-consumer-core-modules`, `unify-duplicate-module-names-core-tui`, and `consolidate-config-modules-into-dir` — those become follow-up cleanups once the split establishes where each module belongs. Execute in this order: (1) finish-io-migration, (2) delete-async-io-bridge, (3) fold-state-into-model-state, (4) rename-core-ui-to-view, (5) this split. Keep `runie-core` as a facade only if external consumers depend on the crate name; otherwise delete and update all `Cargo.toml` edges. The `build.rs` line/complexity limits should tighten on `runie-domain` (pure code) and may need relaxation on `runie-io` actor bodies — decide per-crate limits explicitly.
