# Fix Dev Tools to Use `build_provider_with_warning` Instead of `build_provider`

**Status**: todo
**Milestone**: R2
**Category**: Tools
**Priority**: P2

## Description

Three dev tools still use `runie_agent::build_provider` (which
**panics on unknown provider key**) instead of
`build_provider_with_warning` (which returns a `Result`):

| Crate | File | Line | Current | Should be |
|---|---|---|---|---|
| `runie-print` | `src/main.rs` | 5, 32 | `build_provider(provider_name, model)` (panics) | `build_provider_with_warning(...)?` (propagates error) |
| `runie-json` | `src/main.rs` | 28, 114 | `build_provider(provider_name, model)` (panics) | `build_provider_with_warning(...)?` (propagates error) |
| `runie-server` | `src/main.rs` | 16, 209, 247 | `build_provider(provider_name, model)` (panics) | `build_provider_with_warning(...)?` (propagates error) |

The `runie-term` binary (the main TUI) was already fixed in commit
`87052015` to use `build_provider_with_warning` and emit
`AgentError` events. The dev tools were left in the panicking state.

## Why This Matters

`build_provider` is defined at `crates/runie-agent/src/lib.rs:35-37`:

```rust
pub fn build_provider(provider: &str, model: &str) -> DynProvider {
    runie_provider::build_provider(provider, model)
}
```

And `runie_provider::build_provider` is:

```rust
pub fn build_provider(provider: &str, model: &str) -> DynProvider {
    build_provider_with_warning(provider, model)
        .0
        .expect("build_provider_with_warning returns Ok or panic — use new() for explicit errors")
}
```

So calling `build_provider("nonexistent", "model")` will:
1. Call `build_provider_with_warning("nonexistent", "model")` which
   returns `Err(ProviderError::UnknownProvider("nonexistent".into()))`
2. The `expect` then panics with the message above

For a dev tool like `runie-print` (a one-shot CLI to print a
response), panicking is unfriendly — the user just sees
"thread 'main' panicked" with no context. The right behavior is
to print a clear error and exit with a non-zero status.

## Acceptance Criteria

- [ ] `runie-print/src/main.rs:32` uses `build_provider_with_warning`
  with `?` to propagate the error
- [ ] `runie-json/src/main.rs:114` uses `build_provider_with_warning`
  with `?` to propagate the error
- [ ] `runie-server/src/main.rs:209` and `:247` both use
  `build_provider_with_warning` with `?` to propagate the error
- [ ] Each tool prints a clear error message (e.g. "Unknown
  provider: xyz") and exits with status 1 on failure
- [ ] `cargo build --workspace` succeeds
- [ ] `cargo test -p runie-print` and `cargo test -p runie-json`
  pass (if any tests exist)
- [ ] Each tool can be invoked with an unknown provider and
  produces a friendly error (no panic, non-zero exit)

## Tests

### Layer 1 — State/Logic
- [ ] `test_runie_print_unknown_provider_exits_with_error` — invoking
  `runie-print` with `RUNIE_MODEL=garbage/echo` (or similar)
  prints "Unknown provider: garbage" to stderr and exits with
  status 1
- [ ] `test_runie_json_unknown_provider_exits_with_error` — same for
  `runie-json`
- [ ] `test_runie_server_unknown_provider_exits_with_error` — same
  for `runie-server` (probably tested via integration test that
  connects to the server)

### Layer 4 — Smoke
- [ ] `./target/release/runie-print "hello" --model garbage/echo`
  exits 1 with a clear error (no panic)
- [ ] `./target/release/runie-json --model garbage/echo < input.json`
  exits 1 with a clear error

## Notes

**Pattern for the fix:**

```rust
// Before
let provider = build_provider(provider_name, model);

// After
let provider = build_provider_with_warning(provider_name, model)
    .unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    });
```

Or for `?` propagation in `fn main() -> Result<()>`:

```rust
let provider = build_provider_with_warning(provider_name, model)?;
```

The choice depends on the existing return type of `main()`:
- `runie-print/src/main.rs` — let me check
- `runie-json/src/main.rs` — let me check
- `runie-server/src/main.rs` — already returns `Result<()>` based on
  my earlier read

**The `build_provider` function itself is fine to keep** — it's
still used by code that wants the panicking behavior (tests
that mock the provider key, dev scripts that hardcode a known
key). The fix is to switch the *call sites* that shouldn't panic.

**Out of scope:**
- Removing the `build_provider` function entirely (separate task;
  some callers may genuinely want the panic for fail-fast behavior)
- Renaming `build_provider` to `build_provider_or_panic` to make
  the panic explicit (clarity, not correctness)
- Refactoring the dev tools themselves (separate tasks)
- The `runie-term` binary already does the right thing; no change
  needed there

**Verification:**
```bash
cargo build --workspace
cargo test --workspace

# Friendly error, no panic
./target/release/runie-print "hello" --model garbage/echo 2>&1
# Expected: "Error: Unknown provider: garbage" and exit code 1
```
