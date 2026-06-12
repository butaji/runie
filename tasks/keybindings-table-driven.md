# Make Keybindings Table-Driven Instead of 35-Arm Match

**Status**: todo
**Milestone**: R2
**Category**: Input & Commands
**Priority**: P1

## Description

`crates/runie-core/src/keybindings.rs` has three repetitive
function-with-many-arms patterns that must be kept in sync with every
new `Event` variant:

1. **`default_keybindings()` (lines 15-74)** — 60 lines of `HashMap`
   insertions. Every new key binding requires editing this map.

2. **`event_from_name(name: &str) -> Option<Event>` (lines 140-185)** —
   35-arm match. Every new `Event` variant requires adding an arm here,
   or the keybinding silently fails to bind.

3. **`validate_key_combo(combo: &str) -> bool` (lines 193-263)** — 71
   lines of `matches!(key, "a" | "b" | ... | "pagedown" | "atfilepicker")`
   listing 60+ valid key names. Adding a new event requires verifying
   the key is listed.

A missed update to any of these three is an invisible bug: a
`Event::Foo` variant that has no `event_from_name` arm means users
cannot bind keys to `Foo` via the `keybindings.json` config file.

The same problem applies to `crates/runie-core/src/event.rs` itself:
the 309-line `Event` enum has 100+ variants, each of which must be
mentioned in `keybindings.rs`.

## Acceptance Criteria

- [ ] A single source of truth exists for "event name string ↔ Event
  variant" — a `static EVENT_NAMES: &[(&str, fn() -> Event)]` table or
  a `FromStr` impl on `Event` (preferred)
- [ ] `event_from_name` is a 3-line lookup: `EVENT_NAMES.iter().find(...).map(...)`
- [ ] `default_keybindings` uses the same table; bindings are declared as `(combo, event_name)` tuples
- [ ] `validate_key_combo` derives valid key names from the same table (or has a separate small `const VALID_KEYS: &[&str]` that's easier to extend)
- [ ] A `compile-time test` (a `const _: () = ...` block) asserts that every `Event` variant has a name in the table — catches drift at compile time
- [ ] Adding a new `Event` variant now requires editing exactly one place (the event declaration) plus optionally adding a default keybinding

## Tests

### Layer 1 — State/Logic
- [ ] `test_every_event_variant_has_a_name` — iterates all `Event` variants via exhaustive match, asserts each has an entry in `EVENT_NAMES`. This is a compile-time test (`const _ = ...`) so it can never drift.
- [ ] `test_every_named_event_roundtrips` — for every entry in `EVENT_NAMES`, `event_from_name(name).is_some()` and the resulting event matches the expected variant
- [ ] `test_default_keybindings_resolves` — every entry in `default_keybindings()` produces a valid `Event` via `event_from_name`
- [ ] `test_validate_key_combo_accepts_known_keys` — every key name in `default_keybindings()` passes `validate_key_combo`
- [ ] `test_invalid_event_name_returns_none` — `event_from_name("Garbage")` is `None`
- [ ] `test_invalid_key_combo_returns_false` — `validate_key_combo("ctrl+💩")` is `false`

### Layer 2 — Event Handling
- [ ] `cargo test -p runie-core --lib keybindings::tests` passes (existing 30+ test cases in `keybindings.rs:268-430`)
- [ ] `cargo test -p runie-term --lib keymap::tests` passes (uses `event_from_name` indirectly via `convert_event`)

## Notes

**The table-driven design:**

```rust
// In crates/runie-core/src/event.rs:

/// All event variants with their canonical string names.
/// Add a new entry here whenever you add a variant to `Event`.
pub const EVENT_NAMES: &[(/* EventName */ &str, /* build */ fn() -> Event)] = &[
    ("Quit", || Event::Quit),
    ("Reset", || Event::Reset),
    ("Submit", || Event::Submit),
    ("Abort", || Event::Abort),
    ("Backspace", || Event::Backspace),
    ("Newline", || Event::Newline),
    ("ScrollUp", || Event::ScrollUp),
    // ... 30 more
];

/// Compile-time check: every `Event` variant must have a name in `EVENT_NAMES`.
/// Add a new entry to the table whenever you add a variant.
const _: () = {
    fn _assert_exhaustive(e: &Event) {
        match e {
            Event::Quit => {}
            Event::Reset => {}
            // ... list all variants
            _ => {}  // Catches a missing arm
        }
    }
    // For extra safety, also assert EVENT_NAMES.len() == variant_count
};
```

A simpler design: implement `FromStr` on `Event`:

```rust
impl std::str::FromStr for Event {
    type Err = EventNameError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Quit" => Self::Quit,
            "Reset" => Self::Reset,
            // ...
            _ => return Err(EventNameError(s.into())),
        })
    }
}
```

But this is still a 35-arm match, just in a different file. The
real win is the **compile-time exhaustive check** that catches
drift. Use `match` with a wildcard that fails to compile when
variants are added — this is a Rust idiom called the
"exhaustive match" pattern.

**`validate_key_combo` and the key name list** — the current 60+
literal `|` in `matches!()` is a maintenance burden. Replace with a
`const VALID_KEYS: &[&str] = &[...]` and use `.contains()`:

```rust
const VALID_KEYS: &[&str] = &[
    "a", "b", ..., "z", "0", ..., "9",
    "backspace", "enter", "escape", "tab",
    "up", "down", "left", "right",
    // ...
];

pub fn validate_key_combo(combo: &str) -> bool {
    let parts: Vec<&str> = combo.split('+').collect();
    if parts.is_empty() || parts.len() > 3 { return false; }
    VALID_KEYS.contains(&parts[parts.len() - 1])
}
```

**Out of scope:**
- Changing the `keybindings.json` format (e.g. to TOML or YAML)
- Adding a runtime key-recording UI ("press a key to bind it")
- Per-key chord support (e.g. `Ctrl+X Ctrl+S` to save)
- Platform-specific key remapping (e.g. `Ctrl+C` on macOS)

**Verification:**
```bash
cargo test -p runie-core --lib keybindings
cargo test -p runie-term --lib keymap

# Catch drift: add a new Event variant and confirm the build fails
# (because EVENT_NAMES is missing an entry)
```
