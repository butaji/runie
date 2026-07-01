# Use strum for hook event parsing

## Status

`done`

**Completed:** 2026-07-01

## Context

`crates/runie-core/src/hooks.rs::parse_event_name` was a manual match over snake/camel-case strings. The `strum` crate is already in the workspace dependencies.

## What was done

1. Added `#[derive(strum::EnumString)]` to `HookEvent`
2. Added `#[strum(serialize = "...")]` attributes for each variant's aliases
3. Replaced the 12-line `parse_event_name` function with:
   ```rust
   fn parse_event_name(name: &str) -> Option<HookEvent> {
       HookEvent::from_str(&name.to_ascii_lowercase()).ok()
   }
   ```
4. Removed unused `serde::de::DeserializeOwned` import
5. Updated the test to call `HookEvent::from_str` directly

### Enum after

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumString)]
pub enum HookEvent {
    #[strum(serialize = "pretooluse", serialize = "pre_tool_use")]
    PreToolUse,
    // ... all variants with serialize attributes ...
    #[strum(serialize = "stop")]
    Stop,
}
```

## Acceptance Criteria

- [x] Derive `EnumString` on `HookEvent` with aliases. — **Done**
- [x] Delete `parse_event_name`. — **Replaced with delegation to `HookEvent::from_str`**
- [x] Unknown names still return `None`. — **Done via `.ok()` on `from_str` result**

## Tests

- `cargo check -p runie-core` passes
- `cargo test -p runie-core --lib -- hooks` — 9 tests pass
- `cargo clippy -p runie-core` has no new warnings
> **Live tmux testing session required:** After the implementation passes unit and E2E tests, run a real terminal tmux session that exercises the changed behavior. The task is not done until the live session succeeds.
