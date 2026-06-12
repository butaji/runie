# /copy persists to file

**Status**: done
**Milestone**: R5
**Category**: Input & Commands

## Description

The `/copy` command previously claimed success but did nothing — it fetched
the assistant text into a local variable and dropped it. Now it writes the
text to `<data_dir>/runie/clipboard.md` (or `$RUNIE_CACHE_DIR/clipboard.md` if
set for tests) and tells the user the path.

The path-forward for true system-clipboard support (e.g. `arboard`) is
deferred — terminal paste from a file is reliable across SSH, headless, and
Wayland-or-X11 systems without a C dependency.

## Acceptance Criteria

- [x] `/copy` writes the last assistant message to `clipboard.md`
- [x] The user is told the file path in the system message
- [x] `/copy` with no assistant message shows an error
- [x] Most recent assistant message is used (not the oldest)
- [x] `$RUNIE_CACHE_DIR` overrides the default location for tests
