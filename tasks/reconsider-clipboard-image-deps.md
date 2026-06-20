# Reconsider clipboard-image deps (arboard + png)

**Status**: todo
**Milestone**: R4
**Category**: Configuration
**Priority**: P2

**Depends on**: none
**Blocks**: none

## Description

`arboard` + `png` were adopted in the done task `adopt-arboard-clipboard` to support pasting images from the clipboard into chat. Reversal argument under the YAGNI posture:

- Two crates for one feature: `arboard` (clipboard access, platform-specific) + `png` (decode).
- Only one consumer: `crates/runie-core/src/clipboard_image.rs` (+ `update/input/text.rs` call site).
- A terminal coding agent's primary paste target is text; image paste is a niche flow.
- `arboard` pulls in `xcb` / `x11` / `wayland` / `core-graphics` platform crates; `png` pulls in `miniz_oxide`.

Either (a) gate the feature behind `#[cfg(feature = "clipboard-image")]` and default it off (so the deps become optional), (b) drop it entirely until a user actually pastes an image in anger, or (c) keep and document a concrete usage justification.

## Acceptance Criteria

- [ ] Decision made: EITHER
  - (a) **Feature-gate** — `clipboard_image` module gated behind `#[cfg(feature = "clipboard-image")]`; `arboard` + `png` move to `[features] clipboard-image = ["dep:arboard", "dep:png"]` as optional deps; default build excludes them; OR
  - (b) **Drop** — `clipboard_image.rs` deleted; `update/input/text.rs` image-paste branch removed; `arboard` + `png` removed from manifests; OR
  - (c) **Keep + document** — a concrete usage justification written into `clipboard_image.rs` module docs.
- [ ] If (a) or (b): default `cargo build --workspace` no longer pulls `arboard`, `png`, or their platform transitive deps.
- [ ] `cargo check --workspace` succeeds with no new warnings.
- [ ] `cargo test --workspace` succeeds (with and without the feature if option a).

## Tests

### Layer 1 — State/Logic
- [ ] `is_image_paste_detected_when_enabled` — with `feature = "clipboard-image"`, the paste path detects an image buffer (mocked).
- [ ] `text_paste_unchanged_when_feature_off` — without the feature, pasting plain text is unaffected.

### Layer 2 — Event Handling
- N/A.

### Layer 3 — Rendering
- N/A.

### Layer 4 — Smoke / Crash
- [ ] `smoke_default_build_excludes_arboard` — `cargo build --workspace` without `--features clipboard-image` succeeds and `Cargo.lock` does not include `arboard`.
- [ ] `smoke_feature_build_compiles` — `cargo build --workspace --features clipboard-image` succeeds (option a only).

## Files touched

- `crates/runie-core/src/clipboard_image.rs` (gate / delete / document)
- `crates/runie-core/src/update/input/text.rs` (gate / remove image-paste branch)
- `crates/runie-core/Cargo.toml` (optional-ize or remove `arboard`, `png`)
- `crates/runie-core/src/lib.rs` (gate `pub mod clipboard_image;`)

## Notes

`adopt-arboard-clipboard` notes say it "replaced subprocess-based clipboard with arboard + png crate" — so this is a revert of a revert. The original subprocess approach (`pbpaste` / `xclip` / `powershell Get-Clipboard`) is a third option if image paste must stay but the deps must go: shell out via `IoActor` for the image bytes, skip `arboard`. That keeps the feature on OS tools per the posture. If option (c), link justification and close as `wontfix`.
