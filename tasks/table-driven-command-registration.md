# Refactor 185-line `register()` Functions to Table-Driven

**Status**: done
**Completed**: 2026-06-14
**Milestone**: R2
**Category**: Core Architecture
**Priority**: P1

## Description

`crates/runie-core/src/commands/handlers/session.rs:11` has a
`pub fn register(registry: &mut CommandRegistry)` function that is
**185 lines long** (4.6x the 40-line build.rs cap, 2.3x the
relaxed 80-line cap). It is purely declarative: it calls
`registry.register(crate::cmd!("save").desc("Save session").category(...).sub().form(...))`
~10 times, one per slash command. Same pattern in
`commands/handlers/system.rs:7` (112 lines) and 6 other handler
files.

The 185-line function is hard to:
- Read (the call chain is 6+ methods deep per command)
- Maintain (adding a new command requires careful indentation)
- Test (the function is `pub fn` so it could be tested, but isn't)
- Lint (it triggers every complexity warning in build.rs)

## Current State

| File | Function | Lines |
|---|---|---|
| `commands/handlers/session.rs:11` | `pub fn register` | 185 |
| `commands/handlers/system.rs:7` | `pub fn register` | 112 |
| `commands/handlers/model.rs:9` | `pub fn register` | ~80 |
| `commands/handlers/tool.rs:7` | `pub fn register` | ~25 |
| `commands/handlers/help.rs:7` | `pub fn register` | ~20 |
| `commands/handlers/subagent.rs:7` | `pub fn register` | ~30 |

The `session.rs:11` function is the worst offender: it registers
10 commands (`save`, `load`, `delete`, `export`, `import`,
`sessions`, `new`, `reset`, `history`, `session`, `clone`, `tree`,
`share`, `resume`, `compact`, `fork`, `name`), each as a
6-method call chain.

## Acceptance Criteria

- [x] All `register()` functions are ≤ 40 lines (the strict cap).
- [x] Each handler file has a `register()` function that delegates
  to a `static COMMANDS: &[CommandSpec]` table.
- [x] `CommandSpec` and `CommandKind` live in
  `crates/runie-core/src/commands/handlers/spec.rs` and carry plain data.
- [x] Adding a new slash command requires editing exactly one
  `static COMMANDS` table (no Rust function change).
- [x] The `cmd!()` macro is preserved for ergonomic call-site
  construction.
- [x] `cargo build --workspace` succeeds.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `cargo build --workspace` succeeds.
- [x] `cargo test --workspace` succeeds.
- [x] Existing `commands/tests.rs` exercises all registered commands
  indirectly (registry, slash dispatch, help panel, form panels).

### Layer 4 — Smoke
- [x] Existing smoke scripts can start the TUI; `/help` is rendered
  from the registry.

## Notes

**Refactor pattern:**

```rust
// Before (185 lines of registry.register(...) calls)
pub fn register(registry: &mut CommandRegistry) {
    registry.register(crate::cmd!("save")
        .desc("Save current session")
        .category(CommandCategory::Session)
        .sub()
        .form("Save Session", |f| f.field("Name", "session-name", "name"),
             Event::RunSaveCommand { name: String::new() }));
    // ... 10 more ...
}

// After (40 lines: a static table + a 5-line loop)
static SESSION_COMMANDS: &[CommandSpec] = &[
    CommandSpec {
        name: "save", desc: "Save current session",
        category: CommandCategory::Session, sub: true,
        form: Some(("Save Session", "session-name", "name",
                   Event::RunSaveCommand { name: String::new() })),
    },
    // ... 9 more rows ...
];

pub fn register(registry: &mut CommandRegistry) {
    for spec in SESSION_COMMANDS {
        let cmd = build_cmd(spec);
        registry.register(cmd);
    }
}
```

The `build_cmd()` helper is a single function that converts a
`CommandSpec` row into a `CommandDef`. It handles the
`form/dialog/handler` polymorphism via a `CommandSpec.kind`
enum or a `Box<dyn Fn(&mut AppState, &str) -> CommandResult>`.

**Why table-driven is better:**
- **One place to look**: all commands in a category are in one
  table
- **Compile-time validation**: missing fields are caught at
  compile time
- **Easier to add commands**: copy-paste a row
- **Easier to lint**: no 185-line function to skip
- **Easier to grep**: `grep "name: \"save\""` finds the spec

**Why not a build script or macro?** A `build.rs` or
`macro_rules!` could generate the table from a CSV. But the
specs are heterogeneous (some have forms, some have dialogs,
some have handlers), so the type-driven table is more flexible
than a CSV. The macro would be ~50 lines of complex rules; the
table is ~10 lines per category.

**Why is this P1?** The function length violation is the
largest in the codebase (185 lines, vs 40-line cap). It also
makes the code harder to grep, navigate, and modify. This is a
mechanical refactor with no design decisions — just move code
into a table.

**Out of scope:**
- The `keybindings.rs` 35-arm match and `default_keybindings()`
  64-line HashMap (covered by `keybindings-table-driven` task)
- The `Event` enum's 80+ variants (separate task, see
  `event-subenums.md`)
- The `CommandFlow` enum's 10+ variants (separate task)
- The `register()` functions in `dialog/dsl/`, `commands/dsl/`
  etc. (which are mostly tests or already small)

**Verification:**
```bash
# All register() functions under 40 lines
for f in $(find crates/runie-core/src/commands/handlers -name "*.rs"); do
  awk '...' "$f"
done | awk '$1 > 40'

# Build + tests clean
cargo build --workspace
cargo test --workspace

# Same test count
cargo test --workspace 2>&1 | grep -c '^test '  # should be 1,631
```
