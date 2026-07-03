# Second Five-Round Architecture & Code Review — Synthesis

**Date:** 2026-06-30  
**Goal:** less code, Pareto (80/20) choices, unification and simplification. Replace custom code with crates/libraries/OS features.

This is a follow-up to the first five-round review and the deep crate-replacement analysis. Five new read-only sub-agents drilled into Provider/agent streaming, TUI/events, Config/auth/sessions, Tool execution/permissions, and Build/workspace/test helpers. Findings were cross-checked with `ctx7` and with patterns from `~/Code/agents/{codex,goose,kimi-code,gptme,aider}`. No code was executed or modified.

**TUI design freeze:** behavior may change; visual element design and composition stay frozen.

---

## 1. Consolidated ranked findings

| P | Finding | Files / lines | Replacement | Est. LOC removed | Risk | Borrow from agents | Task |
|---|---------|---------------|-------------|------------------|------|--------------------|------|
| P0 | Bash execution triplicated + `child.kill()` | `tool/bash.rs`, `actors/io/ractor_io.rs`, `update/tools.rs` | single `runie_core::shell` module + `command-group` + `shell-words` | 200–250 | Medium | Goose process-group hygiene | **new** `unify-bash-execution-into-single-shell-module-with-command-group.md` |
| P0 | Permission engines not actually unified | `permissions/mod.rs`, `permissions/rules.rs`, `agent/src/actor.rs` | single `PermissionSet` engine | 300–400 | High | Goose YAML allow/ask/deny | **new** `unify-permission-engines-into-ruleset.md` |
| P0 | `verify-tests.sh` brittle grep + no real timeout | `scripts/verify-tests.sh` | `cargo-nextest` + `.config/nextest.toml` | 90 bash → 40 TOML | Low | Codex nextest config | **new** `replace-verify-tests-with-cargo-nextest.md` |
| P0 | Dead `runie-core/src/testing/` module | `runie-core/src/testing/*.rs` | delete | ~270 | Low | n/a | **new** `delete-dead-runie-core-testing-module.md` |
| P0 | Bash safety heuristic bypassable | `bash_safety.rs` | small regex + OS sandbox (`landlock`/`sandbox-exec`/job objects) | 200 heuristic → 30 + profiles | High | Codex MCP sandbox | **new** `replace-bash-safety-heuristic-with-os-sandboxing.md` |
| P0 | Event taxonomy files hand-generated | `event/generated/*.rs`, `taxonomy.json`, `scripts/generate-event-taxonomy.sh` | real `build.rs` generator or `strum` derives | ~1,440 → ~200 | Medium | Codex schema-driven enums | existing `collapse-event-intent-kind-taxonomies.md`, `use-strum-for-event-intent-names.md` |
| P0 | `tui-textarea` unused despite being a dep | `Cargo.toml`, `ui/input.rs`, `actors/input/messages.rs`, `update/input/*` | `tui-textarea::TextArea` | 800–1,100 | Medium | prompt_toolkit/aider | existing `replace-custom-tui-widgets-with-ratatui-ecosystem.md` |
| P1 | Tool dispatch duplicated in three places | `tool_runner.rs`, `turn/mod.rs`, `headless/mod.rs` | single `ToolRegistry` | 80–120 | Low-Medium | Goose MCP registry | **new** `unify-tool-registry-and-dispatch.md` |
| P1 | `edit_file` + `hashline_edit` two edit models | `tool/edit_file.rs`, `harness_skills/hashline_edit.rs` | patch-based edits via `diffy`/`similar` | ~150 | Medium | aider/gptme patch tools | **new** `unify-edit-file-and-hashline-edit-on-patch-based-edits.md` |
| P1 | Custom `ToolDef` permission constants vs MCP annotations | `tool/schema.rs`, `tool/mod.rs` | `rmcp::model::ToolAnnotations` | ~30 | Low-Medium | MCP standard | **new** `use-mcp-tool-annotations-instead-of-custom-permission-constants.md` |
| P1 | Duplicate `ENV_LOCK` statics | multiple `tests/*.rs` | centralize in `runie-testing` | ~5 statics | Low | Codex common test lib | **new** `centralize-env-lock-in-runie-testing.md` |
| P1 | `register_handler!`, `theme_color!`, `style_fn!` macros | `commands/dsl/handlers/mod.rs`, `theme/colors.rs`, `theme/styles.rs` | plain functions | ~80–100 | Low | Codex typed builders | **new** `replace-register-handler-and-theme-macros-with-functions.md` |
| P1 | Harness skills split commands with `split_whitespace` | `harness_skills/verification_loop.rs`, `startup_context.rs` | `shell-words` | ~10–15 | Low | standard | **new** `use-shell-words-in-harness-skills.md` |
| P1 | MiniMax fixture loader macro + manual match | `runie-testing/src/fixtures/minimax.rs` | `include_dir!` / `LazyLock<HashMap>` | ~40 | Low | Codex fixtures | **new** `use-include-dir-for-minimax-fixtures.md` |
| P1 | `ModelProvider.api_key` still plaintext `String` | `config/mod.rs:57` | remove field; resolve via keyring/env | ~20–30 | Medium | Goose secret separation | **new** `remove-modelfromprovider-api-key-field-and-use-keyring.md` |
| P1 | `SessionIndex` still read despite unified store | `session/index.rs`, `session_handlers.rs:255-257` | delete index, merge header/metadata | ~200–300 | Medium | Goose/Codex single source | **new** `consolidate-session-metadata-and-delete-sessionindex-read-path.md` |
| P1 | Trust/auth file persistence unsafe | `trust.rs:37-44`, `auth/storage.rs:146-159` | `fs2` lock + atomic temp+rename + `0o600` | small | Low | Goose `write_secrets_file` | **new** `make-trust-and-auth-file-persistence-atomic-and-restricted.md` |
| P2 | `ConfigActorState` wraps `Config` in `Mutex` | `actors/config/config_handle.rs`, `ractor_config.rs` | use `&mut state` | ~30 | Low | standard ractor | **new** `remove-configactorstate-unnecessary-mutex.md` |
| P2 | Keymap modifier dispatch tables | `keymap.rs:116-197` | `HashMap<crokey::KeyCombination, CoreEvent>` | ~150 | Low | ecosystem standard | existing `replace-keymap-modifier-tables-with-crokey-map.md` |
| P2 | Dialog list rendering custom | `popups/panel/list.rs`, `panel/mod.rs` | `ratatui::widgets::List` | ~250 | Medium | Ratatui standard | existing `replace-custom-tui-widgets-with-ratatui-ecosystem.md` |
| P2 | Custom form field rendering | `popups/panel/form.rs` | `tui-textarea` single-line / `tui-input` | ~350 | Medium | n/a | existing `replace-custom-tui-widgets-with-ratatui-ecosystem.md` |
| P2 | Dirty-flag redraw model without coalescing | `ui_actor.rs`, `update/**` | drop `dirty` or add `FrameRateLimiter` | ~100 | Low-Medium | Codex frame rate limiter | existing tasks + note in doc |
| P2 | Input history JSONL unbounded/locked | `input_history.rs` | `fs2` lock + cap or SQLite | ~80 | Low | Codex history | existing `fuzzy-history-search-with-sublime-fuzzy.md` |
| P2 | Permission dialog input filtering duplicated | `update/input/mod.rs`, `ui_actor.rs` | single `InputMode` | ~80 | Low | Kimi modal coordinator | existing `refine-permission-dialog-key-handling.md` |
| P2 | Terminal capability detection over-specified | `terminal/caps/detect.rs` | drop brand tables; keep `supports-color` | ~350 | Low-Medium | standard | existing `simplify-terminal-capability-detection.md` |
| P2 | Word-wrapping adapter around `textwrap` | `layout.rs`, `message/wrap.rs` | `textwrap::Options` | ~100 | Low | Codex | existing wrapping tasks |
| P2 | `file:line:col` parser hand-rolled | `location.rs` | `regex` | ~100 | Low | Codex | existing `replace-location-parser-with-regex.md` |
| P2 | `which` already fixed in dirty files | `tool/format.rs` | `which` crate | ~25 | Low | Goose | done |
| P2 | `ApprovalRegistry` custom mutex map | `permissions/approval_registry.rs` | move into ractor state | ~60 | Low | standard ractor | existing permission actor task |
| P2 | `ToolDef` constraints custom DSL | `tool/constraints.rs` | `jsonschema`/`schemars` | ~300 | Low-Medium | MCP schemas | provider task |
| P3 | Mock provider PRNG | `provider/src/mock.rs` | `fastrand`/`rand` | ~15 | Low | standard | minor |
| P3 | `DynProvider` backward-compat wrapper | `provider/src/lib.rs` | migrate to `BuiltProvider` | ~80 | Low | Codex | minor |
| P3 | `find_definitions` language heuristics | `tool/find_definitions.rs` | regex table or tree-sitter | 80–100 | Low-Med | Codex/Goose | minor |
| P3 | `runie-testing` macros.rs misnamed | `runie-testing/src/macros.rs` | rename | trivial | Low | n/a | minor |
| P3 | `runie-testing/timeout.rs` thin wrapper | `runie-testing/src/timeout.rs` | `tokio::time::timeout` | ~50 | Low | n/a | minor |
| P3 | `bacon.toml` dead env knob | `bacon.toml:60-65` | remove or honor | trivial | Low | n/a | minor |

---

## 2. `ctx7` highlights for new crates

| Crate | Why it fits | Evidence |
|-------|-------------|----------|
| `cargo-nextest` | Real per-test timeouts, retries, groups, CI integration. | `slow-timeout = { period = "120s", terminate-after = 5 }` via profile overrides. |
| `landlock` | Linux sandboxing without superuser; restrict FS access. | `Ruleset::default().handle_access(...).create()?.add_rule(PathBeneath::new(...))?.restrict_self()?` |
| `command-group` | Reliable cross-platform process-group kill for bash timeouts. | (not in ctx7; use crate docs/training knowledge) |

---

## 3. Cross-agent learnings

- **Codex:** uses `cargo-nextest` with timeouts/retries, `ConfigEdit` + `toml_edit`, `keyring-store` trait, lossless/best-effort event tiering, `FrameRateLimiter`, and schema-driven generated enums.
- **Goose:** centralizes `Config::get_secret`, uses `sqlx`+SQLite for sessions with migrations, process-group hygiene for subprocesses, `ignore` for file walking, `git2` for repo metadata, and a single YAML permission list.
- **kimi-code:** modal coordinator for stacked dialogs/permissions; recorded vs live-only event split.
- **gptme/aider:** regex-based tool-marker stripping; patch-based edits.

**Takeaways for Runie:**
- Adopt `cargo-nextest` immediately for real test timeouts.
- Move trust/auth/config writes to atomic, restricted, locked files (Goose pattern).
- Unify permission into one ruleset engine (Goose pattern).
- Unify bash execution with process-group kill (Goose hygiene).
- Delete dead test modules before adding more helpers.

---

## 4. Recommended execution order

1. **Delete/consolidate first:** dead testing module, duplicate `ENV_LOCK`, unused deps.
2. **Adopt `cargo-nextest`:** immediate CI/test reliability win.
3. **Safety/security:** atomic/restricted trust/auth files; bash sandboxing.
4. **Config/auth:** remove `api_key` field, centralize credential resolver, `toml_edit`, `figment`.
5. **Permissions:** unify ruleset engine.
6. **Tooling:** single shell module, unified tool registry, patch-based edits.
7. **Sessions:** delete lingering `SessionIndex`, merge metadata types.
8. **TUI:** `tui-textarea` input, `List`/`crokey`/redraw coalescing (visual-freeze).
9. **Provider stack:** spike `async-openai`/`rig-core` adapter, fold lifecycle, unify accumulators.
10. **Build/workspace:** real event taxonomy generator, move build linter to CI, centralize workspace deps.

---

## 5. New tasks created in this pass

1. `unify-bash-execution-into-single-shell-module-with-command-group.md`
2. `unify-permission-engines-into-ruleset.md`
3. `replace-verify-tests-with-cargo-nextest.md`
4. `delete-dead-runie-core-testing-module.md`
5. `replace-bash-safety-heuristic-with-os-sandboxing.md`
6. `unify-tool-registry-and-dispatch.md`
7. `unify-edit-file-and-hashline-edit-on-patch-based-edits.md`
8. `use-mcp-tool-annotations-instead-of-custom-permission-constants.md`
9. `centralize-env-lock-in-runie-testing.md`
10. `replace-register-handler-and-theme-macros-with-functions.md`
11. `use-shell-words-in-harness-skills.md`
12. `use-include-dir-for-minimax-fixtures.md`
13. `remove-modelfromprovider-api-key-field-and-use-keyring.md`
14. `consolidate-session-metadata-and-delete-sessionindex-read-path.md`
15. `make-trust-and-auth-file-persistence-atomic-and-restricted.md`
16. `remove-configactorstate-unnecessary-mutex.md`

---

## 6. File inventory

- This synthesis: `docs/superpowers/plans/2026-06-30-second-five-round-architecture-review.md`
- Prior reviews: `2026-06-30-five-round-architecture-review.md`, `2026-06-30-crate-replacement-deep-dive.md`
- Roadmap: `2026-06-28-runie-cleanup-roadmap.md`
- Tasks: `tasks/*.md`
