# Reconsider clipboard-image deps (arboard + png)

**Status**: done
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

**Decision: (b) Drop** — `clipboard_image.rs` deleted; `update/input/text.rs` image-paste branch removed; `arboard` + `png` removed from manifests.

Rationale:
- Two crates for one feature: `arboard` (clipboard access, platform-specific) + `png` (decode).
- Only one consumer: `clipboard_image.rs` (+ `text.rs` call site).
- A terminal coding agent's primary paste target is text; image paste is a niche flow.
- `arboard` pulls in `xcb` / `x11` / `wayland` / `core-graphics` platform crates; `png` pulls in `miniz_oxide`.

The `PasteImage` event and keybindings remain but now flash to indicate the feature is not supported.

## Acceptance Criteria

- [x] Decision made: (b) **Drop** — `clipboard_image.rs` deleted; `update/input/text.rs` image-paste branch removed; `arboard` + `png` removed from manifests.
- [x] Default `cargo build --workspace` no longer pulls `arboard`, `png`, or their platform transitive deps.
- [x] `cargo check --workspace` succeeds with no new warnings.
- [x] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [x] `text_paste_unchanged_when_feature_off` — without the feature, pasting plain text is unaffected (existing tests pass).

### Layer 2 — Event Handling
- [x] `paste_image_event_handled_gracefully` — `PasteImage` event now triggers flash (no-op).

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [x] `smoke_default_build_excludes_arboard` — `cargo build --workspace` succeeds and `Cargo.lock` does not include `arboard`.

## Files touched

- `crates/runie-core/src/clipboard_image.rs` — deleted
- `crates/runie-core/src/update/input/text.rs` — removed `paste_image` function
- `crates/runie-core/src/update/input/mod.rs` — added `handle_paste_image` handler that flashes
- `crates/runie-core/Cargo.toml` — removed `arboard = "3.6"` and `png = "0.17"`
- `crates/runie-core/src/lib.rs` — removed `pub mod clipboard_image;` and `pub use clipboard_image::read_clipboard_image;`

## Notes

The keybindings for `Ctrl+V` (non-Windows) and `Alt+V` (Windows) still exist and emit `PasteImage` events. They now trigger a flash to indicate the feature is not supported. Users who need image paste can use the subprocess-based approach (`pbpaste` / `xclip` / `powershell Get-Clipboard`) via the tool system in a future iteration.
