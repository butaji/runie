# Sync README, REVIEW, and FEATURE_PARITY With Actual Codebase

**Status**: todo
**Milestone**: R3
**Category**: Configuration
**Priority**: P2

## Description

The user-facing and review documents are out of sync with the actual
codebase. Reading them gives a wrong impression of what the project
is, what it supports, and what its constraints are.

Specific mismatches identified in the code review:

- `README.md` "Crates" table lists `runie-cli`, `runie-tools`,
  `runie-ai` as first-class workspace members. They exist on disk but
  are not in `Cargo.toml`'s `[workspace] members`. (Resolved if
  `wire-orphan-crates` lands; this task picks up the doc edits.)

- `README.md` says "Supports multiple providers: OpenAI, Anthropic,
  Google, MiniMax, Rig". `AnyProvider` only knows Mock + OpenAI. The
  registry lists 13. Resolved if `anyprovider-dynamic-dispatch`
  lands.

- `README.md` documents 9 slash commands + aliases. The actual
  registry (`commands/dsl/builder.rs` + `commands/handlers/*`)
  defines 30+ commands. `/copy` and `/cost` are mentioned in README
  but their handlers don't exist in the code I've seen.

- `README.md` says "8 crates" in the architecture diagram. Reality
  varies (8 in workspace before wire-orphan-crates, 14 on disk).

- `REVIEW.md` lists 14 issues, of which items 1-3, 4 (now
  `update/mod.rs` is 1926 lines, not 623), and 10 (now
  `VisibleRegion` still dead) have not been addressed.

- `IMPL_PLAN.md`, `REFACTOR_PLAN.md`, `FEATURE_PARITY.md`,
  `EXECUTE.md` are 4 plan docs whose relation to the current state
  is unclear.

- `bacon.toml` and `dev.sh` reference crates and paths that may no
  longer exist.

## Acceptance Criteria

- [ ] `README.md` "Architecture" section accurately describes the
  current crate set (whatever is in `[workspace] members` after
  `wire-orphan-crates` lands)
- [ ] `README.md` "Model Support" table lists only providers that
  actually work end-to-end (after `anyprovider-dynamic-dispatch`
  lands, this should be 13; before, only 2)
- [ ] `README.md` "Slash Commands" table matches
  `commands/handlers/*::register` exactly. The `/copy` and `/cost`
  commands are either implemented and documented, or removed from
  the docs
- [ ] `README.md` "Keyboard Shortcuts" table matches
  `keybindings::default_keybindings` exactly
- [ ] `REVIEW.md` is updated to reflect the *current* state (the
  issues from the prior review that are still open, the issues that
  have been resolved, and the new issues from the most recent
  review)
- [ ] `EXECUTE.md`, `IMPL_PLAN.md`, `REFACTOR_PLAN.md`,
  `FEATURE_PARITY.md` are either:
  - (a) Updated to reflect the current state, OR
  - (b) Moved to a `docs/archive/` subdirectory with a one-line
    header noting the date and the plan it captured
- [ ] `bacon.toml` and `dev.sh` reference only existing files and
  crates
- [ ] No document references `runie-cli`, `runie-tools`, `runie-ai`
  as built-in if they're not in the workspace; or they're wired
  (per `wire-orphan-crates`) and the docs are then correct

## Tests

### Layer 1 — State/Logic
- [ ] A new `crates/runie-core/src/tests/doc_sync.rs` test asserts:
  - The set of registered slash commands (from `CommandRegistry::list()`) matches the table in `README.md`
  - The default keybindings map matches the "Keyboard Shortcuts" table in `README.md`
  - The provider registry matches the "Model Support" table in `README.md`

The test reads the README.md at build time, parses the markdown
tables, and asserts the entries match the runtime data.

### Layer 2 — Event Handling
- [ ] N/A (this task is documentation, not behavior)

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Smoke
- [ ] `./dev.sh` runs end-to-end (verifies the script references are correct)
- [ ] `cargo bacon` (if installed) starts without errors referencing missing paths

## Notes

**Doc-test approach** — Rust has built-in support for example tests
via `///` comments and `cargo test --doc`. The existing
`commands/dsl/mod.rs:5-30` docstring example is a starting point.
Add a CI step that runs `cargo test --doc` to catch doc drift.

**Markdown parsing for the test** — use a minimal hand-rolled parser
(e.g. split lines on `|` and trim) rather than pulling in a
`pulldown-cmark` dependency. The goal is to catch obvious drift, not
to validate markdown syntax.

**`REVIEW.md` rewrite** — the prior review had 14 items. Of those:
- #1, #2, #3 (update.rs 623 lines, AppState 28 fields, finish_turn):
  not addressed; the file is now 1926 lines
- #4 (append_response O(n)): unknown, needs measurement
- #5, #6, #7, #8 (caching issues): not addressed
- #9 (test scattering): not addressed (this is `consolidate-tui-tests`)
- #10 (VisibleRegion dead): not addressed (this is `snapshot-dead-code`)
- #11 (action_text impedance): not addressed
- #12 (dev.sh): may still be an issue
- #13 (no e2e for real provider): not addressed
- #14 (ScrollUp/Down naming): not addressed

After this task lands, `REVIEW.md` should be a historical snapshot
of the prior review plus a delta: "from prior review, items X, Y, Z
remain; new items from this review are A, B, C".

**Out of scope:**
- Generating API documentation from rustdoc (already happens; this
  task is about the *non-rustdoc* docs)
- Translating the docs to other languages
- Adding a docs site (mdbook, etc.) — separate task if needed
- Reconciling `bacon.toml` and `dev.sh` with each other

**Verification:**
```bash
# Doc-sync test passes
cargo test -p runie-core --lib tests::doc_sync

# The test catches drift
# (modify a command name in the code, confirm the test fails,
# update the README, confirm it passes)

# All existing tests still pass
cargo test --workspace

# Scripts run
./dev.sh
```
