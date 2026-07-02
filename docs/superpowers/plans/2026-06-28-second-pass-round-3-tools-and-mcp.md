# Round 3 — Tool Execution & MCP Integration

## Findings

### 1. Manual tool dispatch despite `ToolDef` trait

- `crates/runie-agent/src/tool_runner.rs:48-62` — `dispatch_tool` is a giant `match` over tool names.
- `crates/runie-core/src/tool/mod.rs:52-63` — `BUILTIN_TOOL_NAMES` is a second SSOT list.

Both should be generated from a single registry. Options: a registry macro, `inventory`/`linkme`, or `rmcp` if runtime MCP servers are first-class.

### 2. MCP config exists but no runtime client

- `crates/runie-core/src/config/mcp.rs` — only config types.
- `crates/runie-core/src/tool/schema.rs:77-91` — `to_mcp_tool` produces `rmcp::model::Tool` schema but no invocation.

If MCP servers are meant to be first-class, wire `rmcp` client/runtime. Otherwise guard/document the config as not-yet-implemented.

### 3. Tool shell-outs are cross-platform fragile

- `crates/runie-agent/src/tool/grep.rs:81-94` — shells out to `rg`/`grep`.
- `crates/runie-agent/src/tool/find.rs:62-126` — shells out to `fd`/`find`.

Replace with `walkdir`/`ignore` + `regex` (or `grep` crate) to avoid quoting bugs and platform dependencies.

### 4. `find_definitions` uses ad-hoc language detection

- `crates/runie-agent/src/tool/find_definitions.rs:39-116,234-246` — per-language `detect_*` helpers with `starts_with` checks.

Replace with `tree-sitter` parsers for supported languages, or at least a single regex table.

### 5. `fetch_docs` lacks timeout/retry/shared client

- `crates/runie-agent/src/tool/fetch_docs.rs:53-72` — uses `reqwest::get` directly.

Use the centralized provider HTTP client and retry policy.

### 6. `edit_file` is brittle

- `crates/runie-agent/src/tool/edit_file.rs:69-70,117-133` — uses `replacen(...,1)` and manual match count.

Use `similar`/`diffy` for patch-based edits, or a shared search/replace helper.

### 7. Text-based tool shim parsing is overgrown

- `crates/runie-core/src/tool/shim/mod.rs:21-228` — four fallback parsers for text tool markup.

Unify non-XML formats behind a tiny JSON normalizer; keep only XML as a distinct shim.

### 8. DSL command registry duplication

- `crates/runie-core/src/commands/dsl/spec.rs:17-29` and `declarative/types.rs:28-87` — two `CommandKind` shapes.
- `crates/runie-core/src/commands/dsl/category.rs:8-51` — `CommandCategory` duplicates `strum::Display`.
- `crates/runie-core/src/commands/dsl/handlers/registry.rs:28-70` — manual `HandlerRegistry` population.

Collapse `CommandKindDef`, remove manual `label()`/`as_str()`, and use `inventory`/`linkme` for handler registration.

## Recommended changes

1. Generate `BUILTIN_TOOL_NAMES` and `dispatch_tool` from a single registry.
2. Wire `rmcp` client if MCP servers are first-class; otherwise remove dead MCP config.
3. Replace `grep`/`find` shell-outs with `walkdir`/`ignore` + `regex`.
4. Use `tree-sitter` for `find_definitions`.
5. Route `fetch_docs` through the centralized HTTP client + retry.
6. Use `similar` for `edit_file`.
7. Simplify text tool shims to JSON + XML paths.
8. Collapse DSL command types and derive category labels.

## Task mapping

| Finding | Task file | Status |
|---------|-----------|--------|
| Single tool registry | `tasks/generate-tool-dispatch-from-single-registry.md` | **new** |
| Wire or remove MCP client | `tasks/wire-rmcp-client-or-remove-mcp-config.md` | **new** |
| Replace grep/find shell-outs | `tasks/replace-grep-find-shellouts-with-walkdir-ignore-regex.md` | **new** |
| Use tree-sitter for definitions | `tasks/use-tree-sitter-for-find-definitions.md` | **new** |
| Centralize fetch_docs HTTP/retry | `tasks/route-fetch-docs-through-central-http-client.md` | **new** |
| Use diff library for edits | `tasks/use-similar-diffy-for-edit-file.md` | **new** |
| Simplify text tool shims | `tasks/simplify-text-tool-shim-parsers.md` | **new** |
| Collapse DSL command types | `tasks/collapse-commandkinddef-into-commandkind.md` | **new** |
