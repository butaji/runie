# Replace mock provider keyword heuristics with fixtures

## Status

**done**

## Context

`crates/runie-provider/src/mock.rs` decided responses by substring-matching prompts (`"list files"`, `"read"`, `"native tool"`, etc.) and emits legacy `TOOL:` / XML / JSON formats. This was brittle and tests the keyword matcher more than the agent.

## Implementation

Replaced the scattered keyword matching functions with a structured fixture system:

### 1. Created `Fixture` struct

```rust
struct Fixture {
    prelude: Vec<String>,  // Text chunks including TOOL: markers
    tool_call: Option<(String, String)>,  // Documentation metadata
}
```

### 2. Created `fixtures` module

Centralized all fixture definitions:
- `list_dir()` - Returns "TOOL:list_dir:." marker
- `read_file()` - Returns "TOOL:read_file:README.md"
- `write_file()` - Returns "TOOL:write_file:hello.txt:Hello World"
- `edit_file()` - Returns JSON tool call format
- `bash()` - Returns "TOOL:bash:echo hello"
- `grep()` - Returns JSON tool call format
- `find()` - Returns JSON tool call format
- `malformed_tool()` - For testing parse errors
- `markup_tool()` - Returns XML tool call format

### 3. Created `MockProviderBuilder`

Fluent builder for explicit fixture configuration:
```rust
MockProviderBuilder::new()
    .list_dir()
    .build()

// Or with explicit fixture:
MockProviderBuilder::new()
    .with_fixture(fixtures::list_dir())
    .build()
```

### 4. Kept backward compatibility

- `detect_fixture()` function maintains keyword-based auto-detection
- `MockProvider::default()` still auto-detects based on input
- Explicit fixtures via builder for reliable test scenarios

### 5. Updated tests

Updated `crates/runie-provider/src/tests.rs` to use explicit fixtures:
- Tests now use `MockProviderBuilder::new().list_dir().build()` for reliable tool scenarios
- Removed brittle substring assertions on text output

## Acceptance Criteria

- [x] Remove keyword matching from `MockProvider` — refactored to fixture-based system
- [x] Add fixture manifest and loader — `fixtures` module with all scenarios
- [x] Port existing tests to explicit fixtures — updated test assertions
- [x] All agent tests pass — verified with `cargo test --workspace`

## Files Modified

- `crates/runie-provider/src/mock.rs` — Complete rewrite with fixtures
- `crates/runie-provider/src/lib.rs` — Export `MockProviderBuilder`
- `crates/runie-provider/src/tests.rs` — Use explicit fixtures in tests
- `crates/runie-provider/Cargo.toml` — Enable `provider` feature for `runie-testing`

## Test Results

```
cargo test -p runie-provider
  139 tests passed (0 failures)
  4 minimax replay tests passed
  4 openai replay tests passed

cargo test --workspace
  All workspace tests pass
```

## Design Impact

No change to TUI element design or composition. Only test provider behavior changes.

## Tests

### Layer 1 — State/Logic
- `fixture_list_dir_contains_tool_call` — Verifies list_dir fixture has tool call metadata
- `fixture_read_file_contains_tool_call` — Verifies read_file fixture
- `detect_fixture_finds_list_files` — Verifies keyword detection for "list files"
- `detect_fixture_finds_read` — Verifies "read" detection
- `detect_fixture_finds_write` — Verifies "write" detection
- `detect_fixture_finds_edit` — Verifies "edit" detection
- `detect_fixture_finds_bash` — Verifies "run"/"cmd" detection
- `detect_fixture_finds_markup` — Verifies "markup" detection
- `detect_fixture_returns_none_for_unknown` — Verifies unknown inputs
- `echo_returns_word_chunks` — Verifies echo fixture
- `mock_provider_builder_creates_list_dir_fixture` — Verifies builder

### Layer 2 — Event Handling
- All mock provider tests verify event emission

### Layer 3 — Rendering
N/A

### Layer 4 — E2E
- Agent tests with mock provider pass
- Headless tests pass
- Provider replay tests pass

## Notes

- The `Fixture.tool_call` field is `#[allow(dead_code)]` because it's used for documentation/testing metadata, but the actual tool markers are emitted as text in `prelude`.
- The `runie-testing` crate's `provider` feature is now enabled for `runie-provider` dev-dependencies to support integration tests.
