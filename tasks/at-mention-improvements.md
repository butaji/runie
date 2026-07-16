# @-mention ranking and fallback

## Objective

Improve `@` file-reference completion ranking and handle empty or stale `fff` results gracefully.

## Agent landscape finding

opencode, gemini-cli, and kimi-code use `fd` or a fast scanner for fuzzy file completion. Runie already uses its own `fff` indexer.

## runie current state

Runie has `Event::AtFilePicker`, fff-backed search, line-range suffix support (`@file.rs:10-50`), and frecency tracking (`record_file_access`). The gap is ranking polish and a clear empty state.

## Required runie changes

- Boost recently used files (frecency) to the top of `@` picker results.
- Show an explicit "No matching files" state when `fff` returns nothing, with a hint to wait for indexing.
- Ensure the picker refreshes asynchronously when `fff` results arrive after the dialog opens.

## Test scenarios

1. **Recently used file appears first**
   - Keys: attach `@file_a.txt`, then open `@` picker and type query matching both `file_a.txt` and `file_b.txt`.
   - Assert: `file_a.txt` ranks first.

2. **Empty result shows helpful state**
   - Keys: open `@` picker and type a query matching nothing.
   - Assert: "No matching files" or similar hint is shown.

3. **Line-range suffix preserved**
   - Keys: type `@Cargo.toml:10-20`, select `Cargo.toml`.
   - Assert: inserted text is `@Cargo.toml:10-20`.

## Edge / negative cases

- `@` picker works even when the project root has no git repository.
- Picker handles very large result sets without blocking input.

## Dependencies

- `file_references`

## Acceptance checklist

- [ ] All scenarios pass with `AppTest::mock()` and files created in the temp workspace.
- [ ] Edge cases are covered.
- [ ] No `sleep()` in resulting Rust tests.
