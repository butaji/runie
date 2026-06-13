# OSC 52 Clipboard Integration

**Status**: in-progress
**Milestone**: R1
**Category**: TUI / Rendering
**Priority**: P0
**Depends on**: terminal-detection

## Description

Runie's `/copy` command currently writes the last assistant response to a
file (`~/.runie/data/clipboard.md`) and tells the user to open it. This is
clunky compared to Grok's native clipboard integration. Modern terminals
support OSC 52 escape sequences, which allow a TUI application to copy text
directly to the system clipboard without external tools — and it works over
SSH and inside tmux.

## What Was Done

- [x] Created `crates/runie-term/src/terminal/clipboard.rs` with:
  - `osc52_clipboard_sequence(text)` — encodes text as an OSC 52 system
    clipboard sequence.
  - `osc52_primary_sequence(text)` — encodes text for the primary selection.
  - `set_terminal_title(title)` — encodes an OSC 0/2 terminal title sequence.
  - `copy_to_clipboard(writer, text)` and `copy_to_primary(writer, text)` —
    write the sequence to a `std::io::Write` and flush.
- [x] Added `base64 = "0.22"` to `crates/runie-term/Cargo.toml`.
- [x] Added `Event::CopyToClipboard(String)` and `Event::CopyLastResponse` to
  `runie_core::Event`.
- [x] Updated `/copy` handler in `commands/handlers/system.rs` to emit
  `CopyToClipboard(text)` instead of writing a file.
- [x] Added `CopyLastResponse` to the default keybindings at `ctrl+o` (was
  previously an unused alias for `OpenExternalEditor` in help text).
- [x] Wired both events in `runie-term/src/main.rs` so that when
  `terminal_caps.clipboard` is true, the OSC 52 sequence is written directly
  to stdout.
- [x] The capability detection from `terminal-detection` gates OSC 52:
  unsupported terminals silently skip the escape sequence.

## Acceptance Criteria

- [x] `cargo build --workspace` succeeds.
- [x] `cargo test -p runie-term terminal` passes.
- [x] `cargo test -p runie-core tests::copy` passes.
- [x] `cargo test -p runie-core keybindings` passes.
- [x] OSC 52 sequence is emitted only when `terminal_caps.clipboard` is true.
- [x] `/copy` emits a `CopyToClipboard` event with the last assistant text.
- [x] `ctrl+o` resolves to `CopyLastResponse`.

## Tests

### Layer 1 — State/Logic
- [x] `osc52_clipboard_prefix_and_suffix` — sequence starts with `ESC ] 52 ; c ;`
  and ends with `ESC \`.
- [x] `osc52_clipboard_base64_payload` — payload is base64-encoded text.
- [x] `osc52_primary_uses_p_selection` — primary selection uses `p`.
- [x] `set_terminal_title_emits_osc_0` — title sequence is correct.
- [x] `copy_to_clipboard_writes_sequence` — writer receives the exact bytes.
- [x] `update_title_writes_sequence` — title writer receives the exact bytes.
- [x] `empty_string_is_encoded` — empty string produces a valid sequence.
- [x] `copy_emits_clipboard_event_with_last_assistant_text` — `/copy` emits
  `CopyToClipboard` with the latest assistant response.
- [x] `copy_uses_most_recent_assistant_message` — most recent assistant wins.
- [x] `copy_event_payload_does_not_include_older_messages` — older messages are
  excluded.
- [x] `copy_with_no_assistant_message_shows_error` — empty state shows error.

### Layer 2 — Event Handling
- [x] `default_keybindings_resolve` — all default keybindings resolve, including
  the new `CopyLastResponse`.

### Layer 3 — Rendering
- [x] No rendering changes required.

### Layer 4 — Smoke
- [ ] Run `./target/release/runie`, type a message, get a response, press
  `ctrl+o` or run `/copy`, and verify the text is on the system clipboard.

## Notes

**No file fallback yet.** When `terminal_caps.clipboard` is false, the event
is currently swallowed. A follow-up can re-introduce the `clipboard.md` file
fallback for unsupported terminals.

**Title helper is unused.** `set_terminal_title` is included as a companion
utility but not yet called from `setup_terminal`. A follow-up task can set
`runie — <cwd>` on startup and update it on session rename.

**Out of scope:**
- Platform clipboard fallback (pbcopy/xclip).
- Auto-copy on turn completion.
- Terminal title updates.

## Verification

```bash
# Compile clean
cargo build --workspace

# Run clipboard tests
cargo test -p runie-term terminal::clipboard
cargo test -p runie-core tests::copy
cargo test -p runie-core keybindings

# Full suites (skip flaky perf test)
cargo test -p runie-core --lib -- --skip test_render_performance_1000_messages
cargo test -p runie-term --lib -- --skip test_render_performance_1000_messages
```
