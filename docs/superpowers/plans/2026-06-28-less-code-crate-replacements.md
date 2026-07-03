# Less Code / Crate Replacements — Five-Round Review Findings

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` or `superpowers:executing-plans` to implement the linked tasks. No code changes in this document itself.

**Goal:** Replace Runie's custom helper code with proven crates and align declarative formats with peer agent projects, following a strict Pareto (80/20) priority order.

**Architecture:** Keep `runie-core` focused on domain logic (actors, events, tool boundary, sessions). Push generic text/path/glob/fuzzy/keybinding/markdown utilities into standard crates. Use `strum` for generated enum metadata and `pulldown-cmark` as the single markdown authority.

**Tech Stack:** `pulldown-cmark`, `pulldown-cmark-frontmatter`, `strum`, `shellexpand`, `etcetera`, `ignore`, `walkdir`, `nucleo-matcher`/`sublime-fuzzy`, `glob`/`globset`, `crossterm`, `tracing`.

---

## How this review was done

- **No code was run.** Findings come from reading the Runie source tree and the four Rust agent projects in `~/Code/agents/`: `goose`, `jcode`, `openfang`, `thClaws`.
- **ctx7** was consulted for current crate documentation (`pulldown-cmark-frontmatter`, `crossterm`).
- Findings are ranked by Pareto impact: biggest line-count reduction and simplest substitution first.

---

## Round 1 — Custom utility helpers (highest Pareto impact)

Custom modules that mirror mature crates:

| Custom module | Replacement crate | Peer precedent |
|---|---|---|
| `crates/runie-core/src/glob.rs` | `glob` or `globset`/`regex` | `jcode` uses `glob`, `goose` uses `regex` |
| `crates/runie-core/src/fuzzy.rs` | `nucleo-matcher` or `sublime-fuzzy` | `jcode`/`goose` rely on dedicated matchers |
| `crates/runie-core/src/path.rs` | `shellexpand` + `std::path::absolute` (1.79+) | `goose` uses `shellexpand`, `openfang` uses `dirs` |
| `crates/runie-core/src/keybindings/mod.rs` `parse_key_combo` | `crossterm::event::KeyEvent` `Display`/`FromStr` | Built into the TUI stack we already depend on |
| `crates/runie-core/src/telemetry.rs` | `tracing` spans/events | `goose`/`openfang` use `tracing` |

**Planned task:** `tasks/replace-custom-helpers-with-crates.md`

**Priority:** P1. These modules are small, self-contained, and have no external API lock-in.

---

## Round 2 — Text / markdown / tool-boundary processing

The tool-boundary code does multiple regex passes over markdown. `pulldown-cmark` already parses the stream once.

| Custom code | Replacement | Rationale |
|---|---|---|
| `tool_markers/strip.rs` multi-pass regex stripper | `pulldown-cmark` event walker | One semantic pass, correct nested-fence handling |
| `resource_loader.rs` custom frontmatter scanner | `pulldown-cmark-frontmatter` + `serde_yaml` | Handles YAML/TOML frontmatter, removes custom delimiter scanning |
| `event/names.rs` + `name.rs` manual name tables | `strum` derives | Removes ~300 lines of mirrors |

**Planned tasks:**
- `tasks/use-pulldown-cmark-for-tool-marker-stripping.md`
- `tasks/use-pulldown-cmark-frontmatter-for-resource-loader.md`
- `tasks/use-strum-for-event-intent-names.md`

**Priority:** P1. Each removes a class of bugs (fence edge cases, frontmatter drift, name typos).

---

## Round 3 — Enums / concurrency / data structures

| Opportunity | Replacement | Notes |
|---|---|---|
| Manual `Event`/`Intent`/`EventKind` taxonomies | Attribute-driven generator + `strum` | Already planned in `collapse-event-intent-kind-taxonomies` |
| Hand-written actor handle maps | `ractor::ActorRef` typed map | Already planned in `collapse-actor-handles-to-typed-map` |
| Any ad-hoc concurrent hash maps | `dashmap` | Used by `openfang`; only adopt if a hotspot appears |
| Ad-hoc caches | `lru` | Used by `goose`; evaluate after profiling |

**Priority:** P2. The actor and event work is already tracked; the cache/concurrent map suggestions are opportunistic.

---

## Round 4 — Infra / OS features

| Custom/implicit behavior | Replacement | Peer precedent |
|---|---|---|
| Manual home-dir/config-dir resolution | `etcetera` (config dirs), `dirs` (home) | `goose` uses `etcetera`, `openfang` uses `dirs` |
| Manual project file traversal | `ignore` + `walkdir` | `goose`/`jcode` use `ignore` for gitignore-aware walking |
| Command lookup (`which`) | `which` crate | Already in `goose` workspace |
| Shell-like path expansion | `shellexpand` | `goose` uses it |
| Custom telemetry event collector | `tracing` | `goose`/`openfang` standardize on `tracing` |

**Priority:** P1 for `etcetera`/`ignore`/`walkdir`/`shellexpand` because they directly simplify the public-API narrowing task; P2 for `tracing` telemetry replacement.

---

## Round 5 — Cross-agent standards and DSL simplification

Lessons from `thClaws` and `OpenFang`:

- **SKILL.md with YAML frontmatter** is the de-facto standard for packaged skills. Runie should align its loader so skills are portable.
- **`.mcp.json`** is a standard MCP server config location. Runie's MCP config should honor it (already partially adopted; verify).
- **AGENTS.md** is used by `thClaws` for project instructions. Runie already has `AGENTS.md`; keep it canonical.
- **Single-crate core** (`thClaws`) is not appropriate for Runie, but the principle of a thin public API is. Push non-domain utilities out of `runie-core`.

**Planned tasks:**
- `tasks/use-pulldown-cmark-frontmatter-for-resource-loader.md` (alignment)
- `tasks/narrow-runie-core-public-api.md` (thin core)

---

## Execution order

1. **P0:** Actor runtime and bootstrap (existing tasks) — do not replace helpers until the actor migration settles.
2. **P1:** `replace-custom-helpers-with-crates` — removes the most code with the least risk.
3. **P1:** `use-strum-for-event-intent-names` after `collapse-event-intent-kind-taxonomies`.
4. **P1:** `use-pulldown-cmark-for-tool-marker-stripping` after the tool-parser shim is stable.
5. **P1:** `unify-declarative-resource-loader`, then `use-pulldown-cmark-frontmatter-for-resource-loader`.
6. **P2:** `narrow-runie-core-public-api` after helpers are deleted or moved.
7. **P3:** `cleanup-small-duplicates-and-dead-code` as the final sweep.

---

## Rejected alternatives

- **Replace `ractor` with another actor crate.** `ractor` is already the chosen crate; switching again is negative-value churn.
- **Add `dashmap`/`lru` speculatively.** Only adopt if a concrete hotspot is measured.
- **Merge all crates into one core crate.** Runie's crate split (core, agent, provider, cli, tui) is healthy; the goal is to thin `runie-core`, not eliminate it.

---

## Files changed by this plan

- New tasks:
  - `tasks/replace-custom-helpers-with-crates.md`
  - `tasks/use-strum-for-event-intent-names.md`
  - `tasks/use-pulldown-cmark-frontmatter-for-resource-loader.md`
  - `tasks/use-pulldown-cmark-for-tool-marker-stripping.md`
- Updated tasks:
  - `tasks/unify-declarative-resource-loader.md`
  - `tasks/replace-legacy-tool-parsers-with-thin-shim.md`
  - `tasks/collapse-event-intent-kind-taxonomies.md`
  - `tasks/narrow-runie-core-public-api.md`
  - `tasks/cleanup-small-duplicates-and-dead-code.md`
- Registry:
  - `tasks/index.json`
- This document:
  - `docs/superpowers/plans/2026-06-28-less-code-crate-replacements.md`
