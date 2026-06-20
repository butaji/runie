# Audit and reduce borrow-checker workarounds (`take`/`mem::take`)

**Status**: todo
**Milestone**: R4
**Category**: Core / State
**Priority**: P3

**Depends on**: consolidate-login-flow-handlers
**Blocks**: none

## Description

`Option::take()`, `std::mem::take`, `std::mem::replace`, and `std::mem::swap` appear 19x across `update/` and `markdown/`. Many are legitimate (`Option::take` is idiomatic for "move out, leave None"), but a cluster signals a design smell: `state.open_dialog.take()` is called 4x in `update/dialog/router.rs`, 3x in `update/dialog/form_handler.rs`, and 2x in `update/login_flow.rs` — each takes the dialog out of `AppState`, mutates it, and puts it back. This repeated take/process/put-back pattern suggests AppState's dialog ownership model fights the borrow checker and deserves a `with_dialog(|d| ...)` helper (or a `DialogRef` accessor). Audit each site, keep legitimate uses, extract a helper for the open_dialog cluster.

## Acceptance Criteria

- [ ] Every `take()` / `mem::take` / `mem::replace` / `mem::swap` site classified as either: (a) **legitimate** (idiomatic Option move, parser buffer flush) with a `// take: <reason>` comment, or (b) **borrow-conflict workaround** targeted for refactor.
- [ ] `state.open_dialog.take()` cluster (router 4x, form_handler 3x, login_flow 2x = 9 sites) either: (a) replaced by a `with_dialog`/`map_dialog` helper that handles the take/process/put-back internally, or (b) each site documented with the specific borrow conflict that prevents a direct `&mut`.
- [ ] `update/session.rs:108` + `update/input/text.rs:365` `mem::take(&mut self.input.input)` — if both drain the same field, extract an `InputState::take_input()` helper.
- [ ] `markdown/blocks.rs` 6x `mem::take` — classify as parser buffer flushes (likely legitimate); add comments.
- [ ] No new public API surface added unless it eliminates ≥3 take() sites.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `with_dialog_helper_preserves_take_semantics` — if a `with_dialog` helper is extracted, the take/process/put-back behavior is identical (dialog state after == before for noop closure).
- [ ] `open_dialog_none_after_take_if_not_replaced` — take() on None leaves None (existing behavior preserved).

### Layer 2 — Event Handling
- [ ] `dialog_router_routes_after_refactor` — existing dialog router tests pass after `with_dialog` extraction.
- [ ] `form_handler_submits_after_refactor` — existing form handler tests pass.
- [ ] `login_flow_panel_ops_after_refactor` — existing login flow tests pass.

### Layer 3 — Rendering
- [ ] N/A — no rendering changes.

### Layer 4 — Smoke / Crash
- [ ] `cargo test --workspace` green confirms no dialog-state corruption from the refactor.

## Files touched

- `crates/runie-core/src/update/dialog/router.rs` — 4 take() sites
- `crates/runie-core/src/update/dialog/form_handler.rs` — 3 take() sites
- `crates/runie-core/src/update/login_flow.rs` (or `login_flow/handlers.rs`) — 2 take() sites
- `crates/runie-core/src/update/session.rs` — 1 mem::take site (input.input)
- `crates/runie-core/src/update/input/text.rs` — 2 mem::take sites (input.input, image_attachments)
- `crates/runie-core/src/update/input/mod.rs` — 1 take() site (permission_request)
- `crates/runie-core/src/markdown/blocks.rs` — 6 mem::take sites (classify + comment)
- `crates/runie-core/src/commands/dsl/flow.rs:46` — 1 take() site (open_dialog)
- `crates/runie-core/src/ui/transform.rs:142` — 1 mem::take site (parser buffer)
- `crates/runie-core/src/diff.rs:161` — 1 mem::take site (parser buffer)
- `crates/runie-core/src/model/state/app_state.rs` — possibly add `with_dialog` helper

## Notes

This is primarily an audit that may produce a small refactor (the `with_dialog` helper). Do not force a refactor if the take() sites are genuinely independent — a helper that's used twice adds indirection for no benefit. The 9-site `open_dialog` cluster is the strongest candidate. The `markdown/blocks.rs` 6x pattern is almost certainly legitimate parser-buffer flushing (move out of mutable buffer, push to result vec) — comment only, do not refactor. Depends on `consolidate-login-flow-handlers` so the 2 login_flow take() sites are in their final location. Rejected alternative: a generic `AppState::take_dialog()` method — rejected because it doesn't solve the put-back step, which is where the borrow conflict lives.
