# Session title and markdown export

## Objective

Add `/title [name]` to rename the current session and `/export-md [path]` to export the transcript as Markdown.

## Agent landscape finding

codex, kimi-code, and goose support renaming threads and exporting transcripts. Runie has `/save`/`/load` for internal format only.

## runie current state

Runie supports `/save`, `/load`, `/fork`, and `/delete` for sessions in an internal format. There is no human-readable export or rename command.

## Required runie changes

- Add `/title [name]` slash command; opens a small form if no name is provided.
- Persist the title in the saved session metadata.
- Add `/export-md [path]` slash command; exports user and assistant messages as Markdown.
- If path is omitted, default to a timestamped file in the session directory.

## Test scenarios

1. **Rename session**
   - Keys: `type `/title refactor auth` press Enter`
   - Assert: `/status` or session list shows new title.

2. **Export to markdown**
   - Keys: `type `/export-md /tmp/chat.md` press Enter`
   - Assert: file exists and contains transcript Markdown.

3. **Default export path**
   - Keys: `type `/export-md` press Enter`
   - Assert: file created in default location with `.md` extension.

## Edge / negative cases

- Export path outside allowed directories is rejected or confirmed.
- Title with special characters is sanitized before persistence.

## Dependencies

- `status-command`
- `session_management` (if a task exists)

## Acceptance checklist

- [ ] All scenarios pass with `AppTest::mock()`.
- [ ] Edge cases are covered.
- [ ] No `sleep()` in resulting Rust tests.
