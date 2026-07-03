# Fourth Five-Round Architecture & Code Review — Synthesis

**Date:** 2026-06-30  
**Goal:** less code, Pareto (80/20) choices, unification and simplification. Replace custom code with crates/libraries/OS features.

Five new read-only sub-agents drilled into:
1. Remaining TUI live bugs / state mutation / event subscription / redraw
2. Session / store / index / replay / persistence edge cases
3. Testing / replay fixtures / harness / mock providers
4. Resource bundling / `include_dir` / file loading / assets
5. Dependency hygiene / workspace / lockfile / small helpers

Findings were cross-checked with `ctx7` (`wiremock`, `rusqlite`, `tempfile`) and patterns from `~/Code/agents/{codex,goose,kimi-code,gptme,aider}`. No code was executed or modified.

**TUI design freeze:** behavior-only; visual element design and composition stay frozen.

---

## 1. Consolidated ranked findings

| P | Finding | Files / lines | Replacement | Est. LOC removed | Risk | Borrow from agents | Task |
|---|---------|---------------|-------------|------------------|------|--------------------|------|
| P0 | Session persistence: non-atomic header rewrite can destroy whole session | `session/persistence/header.rs:40-48`, `session/store.rs:86,114-116,218` | SQLite (`rusqlite`) or at minimum atomic temp+rename | ~1,000 if SQLite | High (data loss) | Goose SQLite sessions | **new** `migrate-session-persistence-to-rusqlite.md` |
| P0 | Session tree/branch state is lost on save/load | `session/mod.rs:32-33`, `session/tree.rs:98-120` | persist tree edges + current branch (SQLite `parent_message_id` or durable event) | ~100 + schema | High | Goose session tree | **new** `persist-session-tree-and-branch-state.md` |
| P0 | Durable events drop rich message/tool data | `event/durable.rs:14-33`, `event/to_durable.rs:46-62`, `session/replay.rs:63-69` | store full `ChatMessage.parts`, tool success/duration | ~50 + format | Medium | Codex durable events | **new** `preserve-rich-message-and-tool-data-in-durable-events.md` |
| P0 | View cache rebuilt twice per snapshot; `cached_gen` unused | `model/cache/mod.rs:220-236,239-277,280-287,296-330` | keep one cache, compare `message_gen`, reuse | ~30–40 + O(n) pass | Low | Codex projections | **new** `fix-view-cache-double-rebuild-and-use-cached-gen.md` |
| P0 | `apply_steering_delivered` / `apply_follow_up_delivered` retain bug drops queued messages | `model/state/turn_projections.rs:144-148,169-173` | fix inverted retain + delete mirrored queue | ~10–120 | Medium | Goose single queue | **new** `fix-turn-projection-retain-bug-and-delete-mirrored-queue.md` |
| P0 | `AuthToken.token` is plain `String`; `secrecy` dep unused | `auth/storage.rs:7-11,88,116` | `secrecy::SecretString` | ~10 + security | Medium | Codex keyring-store | **new** `use-secrecy-secretstring-in-authtoken.md` |
| P0 | `MockProvider` keyword heuristics are unmaintainable | `runie-provider/src/mock.rs:71-217` | fixture/replay provider + `include_dir` | ~120–150 | Medium | kimi-code recorded events | **new** `replace-mock-provider-keyword-heuristics-with-fixtures.md` |
| P1 | `AgentCommand` constructed manually ~25 times | `runie-agent/src/tests/**/*.rs`, `subagent.rs`, `actor.rs`, `runie-testing/src/runner.rs` | test builder in `runie-testing` | ~200–250 | Low | Codex test helpers | **new** `add-agent-command-test-builder.md` |
| P1 | Provider API tests use raw `std::net::TcpListener` | `runie-provider/src/tests.rs:293-494` | `wiremock` | ~120–150 | Low-Medium | Codex wiremock | **new** `adopt-wiremock-for-provider-api-tests.md` |
| P1 | Event assertions duplicated (`capture_events` + manual `Arc<Mutex<Vec>>`) | `runie-testing/src/replay_provider.rs`, `runie-agent/src/tests/**/*.rs` | assertion helpers in `runie-testing` | ~60–80 | Low | Goose test-support | **new** `add-event-assertion-helpers-to-runie-testing.md` |
| P1 | `MockToolSkill` only supports canned success | `runie-testing/src/mock_tool_skill.rs:16-43` | builder with errors/sequencing/expectations | ~30 helper + tests | Low-Medium | mockall-style | **new** `extend-mock-tool-skill-with-errors-and-sequencing.md` |
| P1 | Fixture corpus only MiniMax; no OpenAI fixtures | `runie-testing/src/fixtures/minimax/` | add `fixtures/openai/` with text/reasoning/tools/error recordings | new files | Medium | Codex fixtures | **new** `add-openai-provider-fixtures.md` |
| P1 | Provider/model registry YAML hard-coded `vec!` of `include_str!` | `provider/registry_data.rs:58-98` | `include_dir!` over `resources/models/` | ~70–90 | Low | Goose catalog | **new** `use-include-dir-for-provider-registry-yaml.md` |
| P1 | Agent manifest SHA validation is custom and fragile | `resources/agents/manifest.json`, `subagents/manifest.rs`, `build.rs:201-231` | `include_dir!` + compile-time fingerprint or delete | ~60–80 + 2 deps | Low | Codex skill fingerprint | **new** `replace-agent-manifest-sha-with-include-dir-fingerprint.md` |
| P1 | `arboard` default features pull image/X11/Wayland deps for text-only use | `Cargo.toml:80`, `runie-core/Cargo.toml:59`, `actors/io/effects/clipboard.rs` | `arboard` with `default-features = false, features = ["wayland-data-control"]` | 0 source; ~10–15 transitive crates | Low | Codex arboard flags | **new** `disable-arboard-image-features-for-text-only-clipboard.md` |
| P2 | `diff/mod.rs` mixes `similar` and `diffy` | `diff/mod.rs:6,207`, `harness_skills/hashline_edit.rs:2,140-142` | unify on one diff crate | ~60–80 + 1 dep | Medium | standard | **new** `unify-diff-generation-and-parsing-on-one-crate.md` |
| P2 | Default theme TOML is 160-line raw string | `runie-tui/src/semantic_tokens.rs:80-161` | move to `resources/themes/runie.toml` + `include_str!` | ~80 moved | Low | Goose prompts as files | **new** `externalize-default-theme-toml-to-resources.md` |
| P2 | `runie-util` is a 2-module micro-crate forcing re-export stubs | `runie-util/src/lib.rs`, `runie-core/src/display_width.rs`, `runie-core/src/labels.rs` | fold into `runie-core` or expand to own all generic helpers | ~3 files | Low-Medium | Codex utility crates | **new** `resolve-runie-util-micro-crate-vs-core-re-exports.md` |
| P2 | `redb` optional dependency is dead weight | `runie-core/Cargo.toml:83` | remove feature and dep | trivial | Low | n/a | **new** `remove-dead-redb-optional-dependency.md` |
| P2 | Default prompts/tools/keybindings are hard-coded strings | `prompts.rs`, `tools list`, `keybindings/defaults.rs` | move to `resources/` and load with `include_str!` | ~50–100 | Low | Goose bundled files | **new** `externalize-default-prompts-tools-and-keybindings-to-resources.md` |
| P2 | `TestRunner` harness unused and polls with `sleep(10ms)` | `runie-testing/src/runner.rs:20-114` | adopt with channel/Notify or delete | ~115 or ~30 | Low | Codex deterministic recv | **new** `fix-or-delete-runie-testing-testrunner.md` |
| P3 | Inline version pins / non-inherited deps | various `Cargo.toml` | normalize to workspace deps | trivial | Low | standard | existing `remove-unused-dependencies-and-normalize-workspace-deps.md` |
| P3 | `CredentialResolver` mutates process env via `dotenvy::dotenv()` | `auth/credential.rs:58-67` | `dotenvy::from_filename_iter` or `figment::Env` | ~10 | Low-Medium | Codex/Goose | existing credential tasks |

---

## 2. `ctx7` / crate documentation highlights

| Crate | Why it fits | Evidence |
|-------|-------------|----------|
| `wiremock` | HTTP mock server for black-box API tests. | `MockServer::start()`, request matchers, `ResponseTemplate`. |
| `rusqlite` | Ergonomic SQLite bindings; bundled feature available. | Standard `Connection`, prepared statements, migrations via `rusqlite_migration`. |
| `tempfile` | Secure cross-platform temp files for atomic writes. | `NamedTempFile::persist()` pattern. |

---

## 3. Cross-agent learnings

- **Codex:** uses `wiremock` for HTTP tests, `include_dir` for bundled resources, schema-driven durable events, explicit test builders, and deterministic channel recv instead of polling.
- **Goose:** SQLite sessions with migrations and tree support; `globset` for permission/file matching; provider catalog as data files; prompts as embedded files.
- **kimi-code:** recorded vs live event split; modal permission coordinator.
- **gptme:** `shlex` tokenization; regex tool-marker stripping.

**Takeaways for Runie:**
- SQLite sessions is the highest-leverage persistence move (fixes data loss, lossy events, tree loss, locking, O(N) listing).
- Bundle resources with `include_dir` instead of hand-maintained lists.
- Build test helpers now while the test surface is still manageable.
- Use crate feature flags to avoid pulling unused OS-specific deps.

---

## 4. Recommended execution order

1. **P0 — SQLite session migration** (or at least atomic header writes + tree/parts persistence if SQLite is deferred).
2. **P0 — fix view cache double rebuild and turn projection retain bug** — live correctness/perf.
3. **P0 — `AuthToken` SecretString** and **MockProvider fixtures** — security + test reliability.
4. **P1 — test helpers** (agent command builder, event assertions, mock tool skill, wiremock, OpenAI fixtures).
5. **P1 — resource bundling** (provider registry, agent manifest, theme, prompts/keybindings).
6. **P1/P2 — dependency hygiene** (arboard features, diff lib unification, `redb` removal, `runie-util` resolution).
7. **P2 — fix/delete TestRunner polling**.
8. **P3 — workspace dep normalization**.

---

## 5. New tasks created in this pass

1. `migrate-session-persistence-to-rusqlite.md`
2. `persist-session-tree-and-branch-state.md`
3. `preserve-rich-message-and-tool-data-in-durable-events.md`
4. `fix-view-cache-double-rebuild-and-use-cached-gen.md`
5. `fix-turn-projection-retain-bug-and-delete-mirrored-queue.md`
6. `use-secrecy-secretstring-in-authtoken.md`
7. `replace-mock-provider-keyword-heuristics-with-fixtures.md`
8. `add-agent-command-test-builder.md`
9. `adopt-wiremock-for-provider-api-tests.md`
10. `add-event-assertion-helpers-to-runie-testing.md`
11. `extend-mock-tool-skill-with-errors-and-sequencing.md`
12. `add-openai-provider-fixtures.md`
13. `use-include-dir-for-provider-registry-yaml.md`
14. `replace-agent-manifest-sha-with-include-dir-fingerprint.md`
15. `disable-arboard-image-features-for-text-only-clipboard.md`
16. `unify-diff-generation-and-parsing-on-one-crate.md`
17. `externalize-default-theme-toml-to-resources.md`
18. `resolve-runie-util-micro-crate-vs-core-re-exports.md`
19. `remove-dead-redb-optional-dependency.md`
20. `externalize-default-prompts-tools-and-keybindings-to-resources.md`
21. `fix-or-delete-runie-testing-testrunner.md`

---

## 6. File inventory

- This synthesis: `docs/superpowers/plans/2026-06-30-fourth-five-round-architecture-review.md`
- Prior reviews: `2026-06-30-five-round-architecture-review.md`, `2026-06-30-crate-replacement-deep-dive.md`, `2026-06-30-second-five-round-architecture-review.md`, `2026-06-30-third-five-round-architecture-review.md`
- Roadmap: `2026-06-28-runie-cleanup-roadmap.md`
- Tasks: `tasks/*.md`
