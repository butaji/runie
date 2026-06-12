# Custom Prompt Templates

**Status**: done
**Milestone**: R3
**Category**: Configuration

## Description

User-defined system prompt overrides. Users can customize the system prompt via config or per-session.

## Architecture

```rust
pub struct PromptTemplate {
    pub name: String,
    pub content: String,
    pub source: PromptSource,
}

pub enum PromptSource {
    BuiltIn,
    UserFile(PathBuf),
    ProjectFile(PathBuf),
}

// In config.toml
// [prompts]
// default = "You are a helpful coding assistant."
// custom = "path/to/prompt.md"
```

## Acceptance Criteria

- [x] `prompts.default` in config.toml sets system prompt
- [x] `prompts.custom` loads from file path
- [x] `/prompt <name>` switches prompt template
- [x] Prompt shown in session info
- [x] Prompts reload with `/reload`

## Tests

### Layer 1
- [x] `prompt_loaded_from_config` — config field parsed
- [x] `prompt_loaded_from_file` — file content read
- [x] `prompt_switch_updates` — /prompt changes system message
