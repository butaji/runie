# Adopt `arboard` for Clipboard Image I/O

**Status**: todo
**Milestone**: R3
**Category**: TUI / Rendering
**Priority**: P1

**Depends on**: (none)
**Blocks**: (none)

## Description

Replace the subprocess-based clipboard-image code in `crates/runie-core/src/clipboard_image.rs` with the `arboard` crate. `arboard` gives a unified cross-platform Rust API for text and image clipboard access and removes the need for `osascript`/`wl-paste`/`xclip` and temporary files.

## Acceptance Criteria

- [ ] `arboard` is added as a dependency.
- [ ] `clipboard_image.rs` uses `arboard` to read image data from the clipboard.
- [ ] macOS, Linux, and Windows are supported (Windows was previously unimplemented).
- [ ] Temporary-file creation is removed.
- [ ] `cargo test --workspace` succeeds.

## Tests

### Layer 1 — State/Logic
- [ ] `clipboard_image_reads_png` — mock or stub test that an image can be read as bytes.

### Layer 2 — Event Handling
N/A.

### Layer 3 — Rendering
N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_paste_image` — copy an image to clipboard, paste in Runie, verify it appears in conversation.

## Files touched

- `crates/runie-core/Cargo.toml`
- `crates/runie-core/src/clipboard_image.rs`

## Notes

- `arboard` may require platform-specific dependencies on Linux (`x11rb` or `wayland` features).
- See `docs/CRATE_DECISIONS.md`.
