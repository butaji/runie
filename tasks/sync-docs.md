# Sync README, REVIEW, and FEATURE_PARITY With Actual Codebase

**Status**: todo
**Milestone**: R3
**Category**: Configuration
**Priority**: P2

## Description

The user-facing and review documents are out of sync with the
actual codebase. Reading them gives a wrong impression of what the
project is, what it supports, and what its constraints are.

The previous `wire-orphan-crates` commit (402943c5) updated
README.md, but the update is partial. Specific mismatches remaining:

- `README.md` "Crates" table is now accurate (8 workspace members)
  per the post-`402943c5` state
- `README.md` "Model Support" table still lists **Anthropic,
  Google, MiniMax, Rig** as supported with their own model lists
  (lines 51-54). The `AnyProvider` was replaced by `DynProvider` per
  `anyprovider-dynamic-dispatch`, which routes all 12 non-Mock
  providers to `OpenAiProvider` parameterized by `base_url`. The
  model lists in the README don't match what's in
  `crates/runie-core/src/provider_registry.rs:107-200`
  (which lists `claude-sonnet-4-6`, `gpt-5`, `gemini-2.5-pro`,
  `deepseek-v4-flash`, etc. — newer than the README)
- `README.md` documents 9 slash commands + aliases. The actual
  registry (`commands/dsl/builder.rs` + `commands/handlers/*`)
  defines 30+ commands. `/copy` and `/cost` are mentioned in
  README but their handlers don't exist
- `commands/dsl/builder.rs:102` docstring example still has the
  stale `build_login_root(state)` signature — see `fix-broken-references`
  open item
- `REVIEW.md` is the previous review (137 lines) and is now
  significantly out of date. Many items it flagged have been
  resolved; new issues have emerged (e.g. `crates/_archive/`
  graveyard, partial `anyprovider-dynamic-dispatch`)
- `FEATURE_PARITY.md` and `EXECUTE.md` are 2 plan docs whose relation
  to the current state is unclear. `IMPL_PLAN.md` and `REFACTOR_PLAN.md`
  have been archived in `docs/archive/`.

## Acceptance Criteria

- [ ] `README.md` "Model Support" table lists the 12 providers
  actually registered in `provider_registry.rs` (anthropic, openai,
  google, deepseek, openrouter, groq, mistral, fireworks, together,
  minimax, moonshotai, xai, ollama) and notes that they all use the
  OpenAI-compatible API
- [ ] `README.md` "Model Support" table mentions that `/model
  <provider>/<model>` switches to any registered provider, and that
  unknown providers return an error (per `anyprovider-dynamic-dispatch`)
- [ ] `README.md` "Slash Commands" table matches
  `commands/handlers/*::register` exactly. The `/copy` and `/cost`
  commands are either implemented and documented, or removed from
  the docs
- [ ] `README.md` "Keyboard Shortcuts" table matches
  `keybindings::default_keybindings` exactly
- [ ] `README.md` "Architecture" diagram accurately describes the
  current crate set (whatever is in `[workspace] members` after
  `archive-remaining-orphans` lands)
- [ ] `REVIEW.md` is updated to reflect the *current* state: items
  from the prior review that have been resolved, items that remain
  open, and new issues from the current code review
- [ ] `commands/dsl/builder.rs:102` docstring example is fixed (the
  `.panel(|state, _| build_login_root(state))` typo)
- [ ] `EXECUTE.md` and `FEATURE_PARITY.md` are either:
  - (a) Updated to reflect the current state, OR
  - (b) Moved to a `docs/archive/` subdirectory with a one-line
    header noting the date and the plan it captured
- [x] `IMPL_PLAN.md` and `REFACTOR_PLAN.md` have been archived in
  `docs/archive/`.
- [ ] `bacon.toml` and `dev.sh` reference only existing files and
  crates

## Tests

### Layer 1 — State/Logic
- [ ] A new `crates/runie-core/src/tests/doc_sync.rs` test asserts:
  - The set of registered slash commands (from
    `CommandRegistry::list()`) matches the table in `README.md`
  - The default keybindings map matches the "Keyboard Shortcuts"
    table in `README.md`
  - The provider registry matches the "Model Support" table in
    `README.md`

The test reads the README.md at build time, parses the markdown
tables, and asserts the entries match the runtime data.

### Layer 2 — Event Handling
- [ ] N/A (this task is documentation, not behavior)

### Layer 3 — Rendering
- [ ] N/A

### Layer 4 — Smoke
- [ ] `./dev.sh` runs end-to-end (verifies the script references
  are correct)
- [ ] `cargo bacon` (if installed) starts without errors referencing
  missing paths

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
- #1, #2, #3 (`update.rs` 623 lines, `AppState` 28 fields, `finish_turn`):
  not directly addressed; the file is now 1901 lines
- #4 (append_response O(n)): unknown, needs measurement
- #5, #6, #7, #8 (caching issues): not addressed
- #9 (test scattering): not addressed (this is
  `consolidate-tui-tests`)
- #10 (VisibleRegion dead): not addressed (this is
  `snapshot-dead-code`)
- #11 (action_text impedance): not addressed
- #12 (dev.sh): may still be an issue
- #13 (no e2e for real provider): not addressed
- #14 (ScrollUp/Down naming): not addressed

Plus items from the current review:
- P0 #1: merge conflicts — **resolved** in `77a605c3`
- P0 #2: `build_login_stack` missing — **resolved** in `0959861e`
  (by archiving the broken caller)
- P0 #3: 6 orphan crates — **partial**: 3 archived
  (`runie-ext`, `runie-ext-macros`, `update/login_flow.rs`),
  3 remain (see `archive-remaining-orphans`)
- P0 #4: `AppState` god-object — not addressed (this is
  `appstate-decomposition`)
- P0 #5: duplicate Panel types — not addressed
- P0 #6: anyprovider closed enum — **partial**: `DynProvider` exists
  but `turn.rs:28` is broken (see `anyprovider-dynamic-dispatch`)
- etc.

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
