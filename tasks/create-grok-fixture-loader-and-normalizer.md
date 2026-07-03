# Create Grok fixture loader and normalizer

## Status

`done`

## Context

No Grok Build fixture module existed; captured Grok output contains non-deterministic timestamps, IDs, and temp paths.

## Implementation

Created `runie-testing/src/fixtures/grok_build.rs` with:

- `include_dir!` fixture loading
- Deterministic sanitization for timestamps, UUIDs, and session IDs
- Unit tests for all sanitization patterns

### Files created/modified

- `crates/runie-testing/src/fixtures/grok_build.rs` — Main fixture loader module
- `crates/runie-testing/src/fixtures/grok_build/sample.sse` — Sample fixture
- `crates/runie-testing/src/fixtures.rs` — Module registration
- `crates/runie-testing/Cargo.toml` — Added regex dependency

### Acceptance Criteria

- [x] Create fixture directory and loader — Done (`include_dir!` pattern)
- [x] Implement sanitizer with stable replacements — Done (timestamps, UUIDs, session IDs)
- [x] Register module in `runie-testing` — Done

## Sanitization Patterns

The sanitizer replaces non-deterministic elements with stable placeholders:

| Pattern | Example Input | Output |
|---------|---------------|--------|
| Timestamps | `2024-06-15T12:30:45Z` | `1970-01-01T00:00:00Z` |
| UUIDs | `a1b2c3d4-e5f6-7890-abcd-ef1234567890` | `00000000-0000-0000-0000-000000000000` |
| Session IDs | `sess_abc123XYZ` | `sess_fixture` |

## Tests

- **Layer 1 — State/Logic:** ✅ Unit tests for sanitizer replacements
- **Layer 4 — E2E:** Fixture loader returns normalized fixture text

### Test Results

```
running 5 tests
test fixtures::grok_build::tests::empty_input ... ok
test fixtures::grok_build::tests::timestamp_sanitization ... ok
test fixtures::grok_build::tests::uuid_sanitization ... ok
test fixtures::grok_build::tests::no_changes_needed ... ok
test fixtures::grok_build::tests::session_id_sanitization ... ok
test result: ok. 5 passed; 0 failed; 0 ignored
```

## Usage

```rust
use runie_testing::fixtures::grok_build;

// Load raw (unsanitized) fixture
let raw = grok_build::raw_fixture("sample.sse");

// Load sanitized fixture for deterministic comparison
let sanitized = grok_build::sanitized_fixture("sample.sse");

// Check if fixture exists
if grok_build::has_fixture("sample.sse") { ... }
```
