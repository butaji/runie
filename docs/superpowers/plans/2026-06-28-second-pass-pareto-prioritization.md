# Second-Pass Review — Pareto Prioritization

**Principle:** every decision must be low-effort, high-impact unless there is a clear architectural blocker. Prefer deleting code, deriving behavior, and replacing custom modules with crates. Do not build new abstractions before the existing ones are collapsed.

## Quick wins (low effort, high impact) — do first

| Task | Effort | Impact | Why |
|------|--------|--------|-----|
| `derive-event-taxonomies-with-strum-or-proc-macro` | low | high | Removes three hand-maintained tables (`kind`, `category`, `EVENT_NAMES`) and prevents variant-name drift. |
| `centralize-provider-error-status-classification` | low | high | One classifier instead of two; fixes SSE vs HTTP divergence. |
| `unify-sse-parsing-on-openai-frame` | low | high | Eliminates duplicated SSE grammar in streaming and replay. |
| `centralize-reqwest-client-and-url-normalization` | low | high | Removes three copies of client builder and URL/key normalization. |
| `derive-chatmessage-builder-with-derive-builder` | low | medium | ~100 lines of boilerplate deleted; prevents field drift. |
| `simplify-modelerror-serde-by-storing-message` | low | medium | Derive `Clone`/`Serialize`/`Deserialize`; remove custom impls. |
| `use-untagged-enum-for-provider-error-bodies` | low | medium | Explicit schema, fewer accessor methods. |
| `consolidate-message-validation-in-one-module` | low | medium | One validation path instead of two. |
| `collapse-commandkinddef-into-commandkind` | low | medium | Removes one conversion layer and manual category labels. |
| `remove-manual-env-overrides-from-config-layers` | low | medium | Use Figment as intended; remove duplicated override logic. |
| `single-config-resolve-default-model` | low | medium | One resolver instead of three. |
| `use-atomic-writes-for-config-and-session-files` | low | medium | Prevents corruption; small helper change. |
| `deduplicate-input-event-mapping-between-forwarder-and-uiactor` | low | medium | One canonical key mapper. |
| `route-permission-clearance-through-permissionactor` | low | medium | Removes direct state write; cleaner event flow. |
| `simplify-text-tool-shim-parsers` | low | medium | Fewer fallback parsers; easier to reason about. |
| `use-textwrap-for-blockquote-word-wrap` | low | low | Deletes custom wrapping code. |
| `fix-throbber-inversion-and-use-throbber-widgets-tui` | low | low | Removes inverted frame math. |

## Medium effort, high impact — do next

| Task | Effort | Impact | Why |
|------|--------|--------|-----|
| `collapse-intent-into-shared-event-payloads` | medium | high | Removes large manual mapping; unifies command vocabulary. |
| `generate-tool-dispatch-from-single-registry` | medium | high | One registry for names and dispatch; unblocks MCP. |
| `replace-grep-find-shellouts-with-walkdir-ignore-regex` | medium | high | Cross-platform, no shell quoting bugs, less code. |
| `use-tree-sitter-for-find-definitions` | medium | high | Accurate definitions; deletes ad-hoc detectors. |
| `use-similar-diffy-for-edit-file` | medium | medium/high | Robust edits; deletes brittle `replacen` logic. |
| `replace-custom-input-box-with-tui-textarea` | medium | high | Deletes custom input widget; gains paste/grapheme support. |
| `replace-custom-form-rendering-with-tui-textarea` | medium | high | Deletes 400-line custom form renderer. |
| `route-tui-autocomplete-through-inputactor-events` | medium | high | Removes direct input-state writes in `UiActor`. |
| `derive-agent-running-flag-from-turnstate-events` | medium | high | Removes UI-side duplicate of turn state. |
| `replay-sessions-via-events-through-appstate` | medium | high | Persistence layer respects event pipeline; unifies replay. |
| `adopt-snapshot-journal-jsonl-pattern` | medium | high | Big performance/reliability win for long sessions. |
| `route-fetch-docs-through-central-http-client` | low/medium | medium | Timeout/retry/share client; small change. |

## High effort or risky — defer until quick wins land

| Task | Effort | Impact | Why |
|------|--------|--------|-----|
| `wire-rmcp-client-or-remove-mcp-config` | high | high | Requires product decision: is MCP first-class? Do not build before tool registry is unified. |
| `remove-or-make-real-providerprotocol-abstraction` | high | high | Requires deciding whether to support non-OpenAI protocols natively. Defer until provider factory is centralized. |
| `use-retryconfig-in-with-retry-or-remove-it` | medium | medium | Blocked by deciding whether per-provider retry config is meaningful. |
| `fix-initial-tui-snapshot-race-after-bootstrap` | medium | medium | Requires careful event ordering; safe to defer. |

## Recommended order

1. Run all **quick wins** in parallel or in any order; they are independent.
2. Run **medium-effort/high-impact** tasks that unblock others:
   - `generate-tool-dispatch-from-single-registry` (unblocks MCP decision)
   - `collapse-intent-into-shared-event-payloads` (unifies event vocabulary)
   - `replay-sessions-via-events-through-appstate` (unblocks persistence cleanup)
3. Run remaining medium tasks.
4. Make the **MCP / provider-protocol** decisions only after the above are done.

## What to avoid

- Do not introduce new macros or proc macros before deriving with `strum`/`derive_builder`.
- Do not add new persistence formats (SQLite was already deferred; stick to JSONL).
- Do not split large files before their internals are simplified; splitting earlier just spreads the same complexity around.
